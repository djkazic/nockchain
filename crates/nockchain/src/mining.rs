use std::str::FromStr;

use kernels::miner::KERNEL;
use nockapp::kernel::checkpoint::JamPaths;
use nockapp::kernel::form::Kernel;
use nockapp::nockapp::driver::{IODriverFn, NockAppHandle, PokeResult};
use nockapp::nockapp::wire::Wire;
use nockapp::nockapp::NockAppError;
use nockapp::noun::slab::NounSlab;
use nockapp::noun::{AtomExt, NounExt};
use nockvm::noun::{Atom, D, T};
use nockvm_macros::tas;
use tempfile::tempdir;
use tracing::{instrument, warn};
use std::sync::Arc;
use tokio::sync::Mutex;

pub enum MiningWire {
    Mined,
    Candidate,
    SetPubKey,
    Enable,
}

impl MiningWire {
    pub fn verb(&self) -> &'static str {
        match self {
            MiningWire::Mined => "mined",
            MiningWire::SetPubKey => "setpubkey",
            MiningWire::Candidate => "candidate",
            MiningWire::Enable => "enable",
        }
    }
}

impl Wire for MiningWire {
    const VERSION: u64 = 1;
    const SOURCE: &'static str = "miner";

    fn to_wire(&self) -> nockapp::wire::WireRepr {
        let tags = vec![self.verb().into()];
        nockapp::wire::WireRepr::new(MiningWire::SOURCE, MiningWire::VERSION, tags)
    }
}

#[derive(Debug, Clone)]
pub struct MiningKeyConfig {
    pub share: u64,
    pub m: u64,
    pub keys: Vec<String>,
}

impl FromStr for MiningKeyConfig {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expected format: "share,m:key1,key2,key3"
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid format. Expected 'share,m:key1,key2,key3'".to_string());
        }

        let share_m: Vec<&str> = parts[0].split(',').collect();
        if share_m.len() != 2 {
            return Err("Invalid share,m format".to_string());
        }

        let share = share_m[0].parse::<u64>().map_err(|e| e.to_string())?;
        let m = share_m[1].parse::<u64>().map_err(|e| e.to_string())?;
        let keys: Vec<String> = parts[1].split(',').map(String::from).collect();

        Ok(MiningKeyConfig { share, m, keys })
    }
}

pub fn create_mining_driver(
    mining_config: Option<Vec<MiningKeyConfig>>,
    mine: bool,
    init_complete_tx: Option<tokio::sync::oneshot::Sender<()>>,
) -> IODriverFn {
    Box::new(move |mut handle| {
        Box::pin(async move {
            let Some(configs) = mining_config else {
                enable_mining(&handle, false).await?;

                if let Some(tx) = init_complete_tx {
                    tx.send(()).map_err(|_| {
                        warn!("Could not send driver initialization for mining driver.");
                        NockAppError::OtherError
                    })?;
                }

                return Ok(());
            };
            if configs.len() == 1
                && configs[0].share == 1
                && configs[0].m == 1
                && configs[0].keys.len() == 1
            {
                set_mining_key(&handle, configs[0].keys[0].clone()).await?;
            } else {
                set_mining_key_advanced(&handle, configs).await?;
            }
            enable_mining(&handle, mine).await?;

            if let Some(tx) = init_complete_tx {
                tx.send(()).map_err(|_| {
                    warn!("Could not send driver initialization for mining driver.");
                    NockAppError::OtherError
                })?;
            }

            if !mine {
                return Ok(());
            }
            let mut next_attempt: Option<NounSlab> = None;
            let mut current_attempt: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
            let mut current_attempt_stop_tx: Option<tokio::sync::oneshot::Sender<()>> = None;

            loop {
                tokio::select! {
                    effect_res = handle.next_effect() => {
                        let Ok(effect) = effect_res else {
                          warn!("Error receiving effect in mining driver: {effect_res:?}");
                        continue;
                        };
                        let Ok(effect_cell) = (unsafe { effect.root().as_cell() }) else {
                            drop(effect);
                            continue;
                        };

                        if effect_cell.head().eq_bytes("mine") {
                            let candidate_slab = {
                                let mut slab = NounSlab::new();
                                slab.copy_into(effect_cell.tail());
                                slab
                            };

                            // If there's an active attempt, send it a stop signal.
                            // This ensures that a new `mine` effect always tries to stop the current work.
                            if let Some(tx) = current_attempt_stop_tx.take() { // Take the sender (making current_attempt_stop_tx None)
                                let _ = tx.send(()); // Send stop signal. Ignore error if receiver already dropped.
                            }

                            // Create a new oneshot channel for the new attempt
                            let (new_stop_tx, new_stop_rx) = tokio::sync::oneshot::channel();
                            // Store the sender for this new attempt
                            current_attempt_stop_tx = Some(new_stop_tx);

                            // If a task is currently running OR `next_attempt` already holds a queued candidate,
                            // then this new candidate becomes the next one to process.
                            // Otherwise, spawn it immediately.
                            if !current_attempt.is_empty() || next_attempt.is_some() {
                                next_attempt = Some(candidate_slab);
                            } else {
                                // No task is running and no next attempt is queued, so spawn immediately.
                                let (cur_handle, attempt_handle) = handle.dup();
                                handle = cur_handle;
                                current_attempt.spawn(mining_attempt(
                                    candidate_slab,
                                    attempt_handle,
                                    new_stop_rx, // <--- Pass the Receiver here
                                ));
                            }
                        }
                    },
                    // This branch fires when a spawned mining_attempt task completes
                    mining_attempt_res = current_attempt.join_next(), if !current_attempt.is_empty()  => {
                        if let Some(Err(e)) = mining_attempt_res {
                            warn!("Error during mining attempt: {e:?}");
                        }

                        // The task has completed, so its stop_tx is no longer relevant.
                        current_attempt_stop_tx = None;

                        // If there's a queued candidate, spawn it now
                        if let Some(candidate_slab) = next_attempt.take() { // Use .take() to consume the value
                            // Create a new oneshot channel for this new task
                            let (new_stop_tx, new_stop_rx) = tokio::sync::oneshot::channel();
                            // Store the sender for this new attempt
                            current_attempt_stop_tx = Some(new_stop_tx);

                            let (cur_handle, attempt_handle) = handle.dup();
                            handle = cur_handle;
                            current_attempt.spawn(mining_attempt(
                                candidate_slab,
                                attempt_handle,
                                new_stop_rx, // <--- Pass the Receiver here
                            ));
                        }
                    }
                }
            }
        })
    })
}

pub async fn mining_attempt(candidate: NounSlab, handle: NockAppHandle, mut stop_rx: tokio::sync::oneshot::Receiver<()>) -> () {
    let snapshot_dir =
        tokio::task::spawn_blocking(|| tempdir().expect("Failed to create temporary directory"))
            .await
            .expect("Failed to create temporary directory");
    let hot_state = zkvm_jetpack::hot::produce_prover_hot_state();
    let snapshot_path_buf = snapshot_dir.path().to_path_buf();
    let jam_paths = JamPaths::new(snapshot_dir.path());

    // Spawns a new std::thread for this mining attempt
    // Wrap Kernel in Arc<Mutex> to allow safe concurrent access within tokio::select!
    let kernel = Arc::new(Mutex::new(
        Kernel::load_with_hot_state_huge(snapshot_path_buf, jam_paths, KERNEL, &hot_state, false)
            .await
            .expect("Could not load mining kernel")
    ));

    tokio::select! {
        // Branch 1: The kernel poke completes
        // Use an async block to acquire the lock and then call poke()
        effects_slab_res = async {
            let k_guard = kernel.lock().await; // Acquire an exclusive lock on Kernel
            // `k_guard` is `MutexGuard<Kernel>`, which derefs to `&mut Kernel`.
            // `poke` takes `&self`, which is compatible with `&mut Kernel` (coercion).
            k_guard.poke(MiningWire::Candidate.to_wire(), candidate).await
        } => {
            let effects_slab = effects_slab_res.expect("Could not poke mining kernel with candidate");
            for effect in effects_slab.to_vec() {
                let Ok(effect_cell) = (unsafe { effect.root().as_cell() }) else {
                    drop(effect);
                    continue;
                };
                if effect_cell.head().eq_bytes("command") {
                    handle
                        .poke(MiningWire::Mined.to_wire(), effect)
                        .await
                        .expect("Could not poke nockchain with mined PoW");
                }
            }
        },
        // Branch 2: A stop signal is received
        _ = &mut stop_rx => {
            // Signal received. Call the async kernel.stop() method and await it.
            // Acquire the lock here as well to ensure exclusive access for stop().
            let mut k_guard = kernel.lock().await;
            let _ = k_guard.stop().await;
            return;
        }
    }
}

#[instrument(skip(handle, pubkey))]
async fn set_mining_key(
    handle: &NockAppHandle,
    pubkey: String,
) -> Result<PokeResult, NockAppError> {
    let mut set_mining_key_slab = NounSlab::new();
    let set_mining_key = Atom::from_value(&mut set_mining_key_slab, "set-mining-key")
        .expect("Failed to create set-mining-key atom");
    let pubkey_cord =
        Atom::from_value(&mut set_mining_key_slab, pubkey).expect("Failed to create pubkey atom");
    let set_mining_key_poke = T(
        &mut set_mining_key_slab,
        &[
            D(tas!(b"command")),
            set_mining_key.as_noun(),
            pubkey_cord.as_noun(),
        ],
    );
    set_mining_key_slab.set_root(set_mining_key_poke);

    handle
        .poke(MiningWire::SetPubKey.to_wire(), set_mining_key_slab)
        .await
}

async fn set_mining_key_advanced(
    handle: &NockAppHandle,
    configs: Vec<MiningKeyConfig>,
) -> Result<PokeResult, NockAppError> {
    let mut set_mining_key_slab = NounSlab::new();
    let set_mining_key_adv = Atom::from_value(&mut set_mining_key_slab, "set-mining-key-advanced")
        .expect("Failed to create set-mining-key-advanced atom");

    // Create the list of configs
    let mut configs_list = D(0);
    for config in configs {
        // Create the list of keys
        let mut keys_noun = D(0);
        for key in config.keys {
            let key_atom =
                Atom::from_value(&mut set_mining_key_slab, key).expect("Failed to create key atom");
            keys_noun = T(&mut set_mining_key_slab, &[key_atom.as_noun(), keys_noun]);
        }

        // Create the config tuple [share m keys]
        let config_tuple = T(
            &mut set_mining_key_slab,
            &[D(config.share), D(config.m), keys_noun],
        );

        configs_list = T(&mut set_mining_key_slab, &[config_tuple, configs_list]);
    }

    let set_mining_key_poke = T(
        &mut set_mining_key_slab,
        &[
            D(tas!(b"command")),
            set_mining_key_adv.as_noun(),
            configs_list,
        ],
    );
    set_mining_key_slab.set_root(set_mining_key_poke);

    handle
        .poke(MiningWire::SetPubKey.to_wire(), set_mining_key_slab)
        .await
}

//TODO add %set-mining-key-multisig poke
#[instrument(skip(handle))]
async fn enable_mining(handle: &NockAppHandle, enable: bool) -> Result<PokeResult, NockAppError> {
    let mut enable_mining_slab = NounSlab::new();
    let enable_mining = Atom::from_value(&mut enable_mining_slab, "enable-mining")
        .expect("Failed to create enable-mining atom");
    let enable_mining_poke = T(
        &mut enable_mining_slab,
        &[
            D(tas!(b"command")),
            enable_mining.as_noun(),
            D(if enable { 0 } else { 1 }),
        ],
    );
    enable_mining_slab.set_root(enable_mining_poke);
    handle
        .poke(MiningWire::Enable.to_wire(), enable_mining_slab)
        .await
}
