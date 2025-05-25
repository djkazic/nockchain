use nockvm::interpreter::Context;
use nockvm::jets::util::slot;
use nockvm::jets::JetErr;
use nockvm::noun::{IndirectAtom, Noun};
use tracing::debug;

use crate::form::fext::*;
use crate::form::poly::*;
use crate::hand::handle::new_handle_mut_felt;
use crate::jets::utils::{noun_to_vec_belt, vec_belt_to_noun, jet_err};
use crate::noun::noun_ext::NounExt;
use crate::utils::*;

pub fn fadd_jet(context: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    let sam = slot(subject, 6)?;
    let a = slot(sam, 2)?;
    let b = slot(sam, 3)?;

    let (Ok(a_felt), Ok(b_felt)) = (a.as_felt(), b.as_felt()) else {
        debug!("a or b not a felt");
        return jet_err();
    };
    let (res_atom, res_felt): (IndirectAtom, &mut Felt) = new_handle_mut_felt(&mut context.stack);
    fadd(a_felt, b_felt, res_felt);

    assert!(felt_atom_is_valid(res_atom));
    Ok(res_atom.as_noun())
}

pub fn fsub_jet(context: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    let sam = slot(subject, 6)?;
    let a = slot(sam, 2)?;
    let b = slot(sam, 3)?;

    let (Ok(a_felt), Ok(b_felt)) = (a.as_felt(), b.as_felt()) else {
        debug!("a or b not a felt");
        return jet_err();
    };
    let (res_atom, res_felt): (IndirectAtom, &mut Felt) = new_handle_mut_felt(&mut context.stack);
    fsub(a_felt, b_felt, res_felt);

    assert!(felt_atom_is_valid(res_atom));
    Ok(res_atom.as_noun())
}

pub fn fneg_jet(context: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    let a = slot(subject, 6)?;

    let Ok(a_felt) = a.as_felt() else {
        debug!("a not a felt");
        return jet_err();
    };
    let (res_atom, res_felt): (IndirectAtom, &mut Felt) = new_handle_mut_felt(&mut context.stack);
    fneg(a_felt, res_felt);

    assert!(felt_atom_is_valid(res_atom));
    Ok(res_atom.as_noun())
}

pub fn fmul_jet(context: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    let sam = slot(subject, 6)?;
    let a = slot(sam, 2)?;
    let b = slot(sam, 3)?;

    let (Ok(a_felt), Ok(b_felt)) = (a.as_felt(), b.as_felt()) else {
        debug!("a or b not a felt");
        return jet_err();
    };
    let (res_atom, res_felt): (IndirectAtom, &mut Felt) = new_handle_mut_felt(&mut context.stack);
    fmul(a_felt, b_felt, res_felt);

    assert!(felt_atom_is_valid(res_atom));
    Ok(res_atom.as_noun())
}

pub fn finv_jet(context: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    let a = slot(subject, 6)?;

    let Ok(a_felt) = a.as_felt() else {
        debug!("a is not a felt");
        return jet_err();
    };
    let (res_atom, res_felt): (IndirectAtom, &mut Felt) = new_handle_mut_felt(&mut context.stack);
    finv(a_felt, res_felt);

    assert!(felt_atom_is_valid(res_atom));
    Ok(res_atom.as_noun())
}

pub fn fdiv_jet(context: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    let sam = slot(subject, 6)?;
    let a = slot(sam, 2)?;
    let b = slot(sam, 3)?;

    let (Ok(a_felt), Ok(b_felt)) = (a.as_felt(), b.as_felt()) else {
        debug!("a or b not felts");
        return jet_err();
    };
    let (res_atom, res_felt): (IndirectAtom, &mut Felt) = new_handle_mut_felt(&mut context.stack);
    fdiv(a_felt, b_felt, res_felt);

    assert!(felt_atom_is_valid(res_atom));
    Ok(res_atom.as_noun())
}

pub fn fpow_jet(context: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    let sam = slot(subject, 6)?;
    let x = slot(sam, 2)?;
    let n = slot(sam, 3)?;

    let (Ok(x_felt), Ok(n_atom)) = (x.as_felt(), n.as_atom()) else {
        debug!("x not a felt or n not an atom");
        return jet_err();
    };
    let n_64 = n_atom.as_u64()?;
    let (res_atom, res_felt): (IndirectAtom, &mut Felt) = new_handle_mut_felt(&mut context.stack);
    fpow(x_felt, n_64, res_felt);

    assert!(felt_atom_is_valid(res_atom));
    Ok(res_atom.as_noun())
}

pub fn fp_fft_jet(ctx: &mut Context, subject: Noun) -> Result<Noun, JetErr> {
    // 1) decode the Hoon list into Vec<Belt>
    let mut coeffs = match noun_to_vec_belt(subject) {
      Ok(v) => v,
      Err(_) => return jet_err(),
    };
    let n = coeffs.len();

    // 2) must be non-empty and a power of two
    if n == 0 || !n.is_power_of_two() {
        return jet_err();
    }

    // 3) compute the “lifted” root for size `n`
    let root = match Belt(n as u64).ordered_root() {
      Ok(r) => r,
      Err(_) => return jet_err(),
    };

    // 4a) bit-reverse permutation
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            coeffs.swap(i, j);
        }
    }

    // 4b) Danielson–Lanczos butterflies
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        // twiddle step = Belt(root.0.pow(expo))
        let expo: u32 = (n / len) as u32;
        let step = Belt(root.0.pow(expo));

        for chunk in coeffs.chunks_exact_mut(len) {
            let mut w = Belt::one();
            for i in 0..half {
                let u = chunk[i];
                let t = chunk[i + half] * w;
                chunk[i]        = u + t;
                chunk[i + half] = u - t;
                w = w * step;
            }
        }

        len <<= 1;
    }

    // 5) pack back into a Hoon list and return
    Ok(vec_belt_to_noun(ctx, &coeffs))
}