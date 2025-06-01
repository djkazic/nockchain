use nockvm::interpreter::Context;
use nockvm::jets::util::slot;
use nockvm::jets::Result;
use nockvm::noun::{IndirectAtom, Noun, D};

use crate::form::math::bpoly::*;
use crate::form::poly::*;
use crate::hand::handle::*;
use crate::hand::structs::{HoonMap, HoonMapIter};
use crate::jets::utils::jet_err;
use crate::noun::noun_ext::NounExt;

use crate::form::mega::{brek, MegaTyp};
use crate::form::math::base::bpow;


fn zero_bpoly() -> BPolyVec {
    BPolyVec::from(vec![0u64])
}

fn ones_bpoly(len: usize) -> BPolyVec {
    BPolyVec::from(vec![1u64; len])
}

pub fn mp_substitute_mega_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let stack = &mut context.stack;

    let [p_noun, trace_evals_noun, height_noun, chal_map_noun, dyns_noun, com_map_noun] =
        sam.uncell()?;

    let Ok(trace_evals) = BPolySlice::try_from(trace_evals_noun) else {
        return jet_err::<Noun>();
    };
    let Ok(height_atom) = height_noun.as_atom() else {
        return jet_err::<Noun>();
    };
    let Ok(height) = height_atom.as_u64() else {
        return jet_err::<Noun>();
    };
    let height_usize = height as usize;

    let Ok(dyns) = BPolySlice::try_from(dyns_noun) else {
        return jet_err::<Noun>();
    };

    let Ok(chal_map) = HoonMap::try_from(chal_map_noun) else {
        return jet_err::<Noun>();
    };
    let Ok(com_map) = HoonMap::try_from(com_map_noun) else {
        return jet_err::<Noun>();
    };

    let mut acc_vec = zero_bpoly();

    let mut p_iter = HoonMapIter::from(p_noun);
    p_iter.try_fold((), |_, n| {
        let [k_noun, v_noun] = n.uncell()?;
        let Ok(k) = BPolySlice::try_from(k_noun) else {
            return jet_err::<()>();
        };
        let Ok(v) = v_noun.as_belt() else {
            return jet_err::<()>();
        };

        if v.0 == 0 {
            return Ok(());
        }

        let poly_len_for_var_com = 4 * height_usize;

        let mut inner_acc_vec = ones_bpoly(k.len());

        for i in 0..k.len() {
            let ter = k.0[i];

            let (typ, idx, exp) = brek(ter);

            match typ {
                MegaTyp::Var => {
                    let var_start_idx = idx * poly_len_for_var_com;
                    let var_end_idx = var_start_idx + poly_len_for_var_com;

                    if var_end_idx > trace_evals.len() {
                        return jet_err::<()>();
                    }
                    let var_slice = &trace_evals.0[var_start_idx..var_end_idx];

                    let hadamard_res_len = inner_acc_vec.len().min(var_slice.len());
                    let (_res_atom, res_poly_slice): (IndirectAtom, &mut [Belt]) =
                        new_handle_mut_slice(stack, Some(hadamard_res_len));

                    for _ in 0..exp {
                        let current_inner_acc_slice = &inner_acc_vec.0;
                        bp_hadamard(current_inner_acc_slice, var_slice, res_poly_slice);
                        inner_acc_vec = BPolyVec::from(res_poly_slice.iter().map(|&b| b.0).collect::<Vec<u64>>());
                    }
                }
                MegaTyp::Rnd => {
                    let rnd_noun = chal_map.get(stack, D(idx as u64)).ok_or_else(|| jet_err::<()>().unwrap_err())?;
                    let Ok(rnd) = rnd_noun.as_belt() else {
                        return jet_err::<()>();
                    };

                    let pow_rnd = bpow(rnd.0, exp);
                    let (_res_atom, res_poly_slice): (IndirectAtom, &mut [Belt]) =
                        new_handle_mut_slice(stack, Some(inner_acc_vec.len()));
                    bpscal(Belt(pow_rnd), &inner_acc_vec.0, res_poly_slice);
                    inner_acc_vec = BPolyVec::from(res_poly_slice.iter().map(|&b| b.0).collect::<Vec<u64>>());
                }
                MegaTyp::Dyn => {
                    if idx >= dyns.len() {
                        return jet_err::<()>();
                    }
                    let dyn_val = dyns.0[idx];

                    let pow_dyn = bpow(dyn_val.0, exp);
                    let (_res_atom, res_poly_slice): (IndirectAtom, &mut [Belt]) =
                        new_handle_mut_slice(stack, Some(inner_acc_vec.len()));
                    bpscal(Belt(pow_dyn), &inner_acc_vec.0, res_poly_slice);
                    inner_acc_vec = BPolyVec::from(res_poly_slice.iter().map(|&b| b.0).collect::<Vec<u64>>());
                }
                MegaTyp::Con => {
                }
                MegaTyp::Com => {
                    let com_noun = com_map.get(stack, D(idx as u64)).ok_or_else(|| jet_err::<()>().unwrap_err())?;
                    let Ok(com_slice) = BPolySlice::try_from(com_noun) else {
                        return jet_err::<()>();
                    };

                    let hadamard_res_len = inner_acc_vec.len().min(com_slice.len());
                    let (_res_atom, res_poly_slice): (IndirectAtom, &mut [Belt]) =
                        new_handle_mut_slice(stack, Some(hadamard_res_len));

                    for _ in 0..exp {
                        let current_inner_acc_slice = &inner_acc_vec.0;
                        bp_hadamard(current_inner_acc_slice, com_slice.0, res_poly_slice);
                        inner_acc_vec = BPolyVec::from(res_poly_slice.iter().map(|&b| b.0).collect::<Vec<u64>>());
                    }
                }
            }
        }

        let (_scaled_atom, scaled_poly_slice): (IndirectAtom, &mut [Belt]) =
            new_handle_mut_slice(stack, Some(inner_acc_vec.len()));
        bpscal(v, &inner_acc_vec.0, scaled_poly_slice);
        let scaled_inner_vec = BPolyVec::from(scaled_poly_slice.iter().map(|&b| b.0).collect::<Vec<u64>>());

        let new_acc_len = acc_vec.len().max(scaled_inner_vec.len());
        let (_new_acc_atom, new_acc_poly_slice): (IndirectAtom, &mut [Belt]) =
            new_handle_mut_slice(stack, Some(new_acc_len));
        bpadd(&acc_vec.0, &scaled_inner_vec.0, new_acc_poly_slice);
        acc_vec = BPolyVec::from(new_acc_poly_slice.iter().map(|&b| b.0).collect::<Vec<u64>>());

        Ok(())
    })?;

    let (_final_res_atom, final_res_poly_slice): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(stack, Some(acc_vec.len()));
    final_res_poly_slice.copy_from_slice(&acc_vec.0);

    let res_cell = finalize_poly(stack, Some(final_res_poly_slice.len()), _final_res_atom);

    Ok(res_cell)
}