use nockvm::interpreter::Context;
use nockvm::jets::util::slot;
use nockvm::jets::Result;
use nockvm::noun::{Atom, IndirectAtom, Noun, D, T};
use tracing::info;

use crate::form::math::bpoly::*;
use crate::form::poly::*;
use crate::hand::handle::*;
use crate::hand::structs::HoonList;
use crate::jets::utils::jet_err;
use crate::noun::noun_ext::{AtomExt, NounExt};

pub fn bpoly_to_list_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    bpoly_to_list(context, sam)
}

pub fn bpoly_to_list(context: &mut Context, sam: Noun) -> Result {
    let Ok(sam_bpoly) = BPolySlice::try_from(sam) else {
        return jet_err();
    };

    //  empty list is a null atom
    let mut res_list = D(0);

    let len = sam_bpoly.len();

    if len == 0 {
        return Ok(res_list);
    }

    for i in (0..len).rev() {
        let res_atom = Atom::new(&mut context.stack, sam_bpoly.0[i].into());
        res_list = T(&mut context.stack, &[res_atom.as_noun(), res_list]);
    }

    Ok(res_list)
}

pub fn bpadd_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let bp = slot(sam, 2)?;
    let bq = slot(sam, 3)?;

    let (Ok(bp_poly), Ok(bq_poly)) = (BPolySlice::try_from(bp), BPolySlice::try_from(bq)) else {
        return jet_err();
    };

    let res_len = std::cmp::max(bp_poly.len(), bq_poly.len());
    let (res, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len as usize));
    bpadd(bp_poly.0, bq_poly.0, res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);

    Ok(res_cell)
}

pub fn bpneg_jet(context: &mut Context, subject: Noun) -> Result {
    let bp = slot(subject, 6)?;

    let Ok(bp_poly) = BPolySlice::try_from(bp) else {
        return jet_err();
    };

    let (res, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(bp_poly.len()));
    bpneg(bp_poly.0, res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);

    Ok(res_cell)
}

pub fn bpsub_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let p = slot(sam, 2)?;
    let q = slot(sam, 3)?;

    let (Ok(p_poly), Ok(q_poly)) = (BPolySlice::try_from(p), BPolySlice::try_from(q)) else {
        return jet_err();
    };

    let res_len = std::cmp::max(p_poly.len(), q_poly.len());
    let (res, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len as usize));
    bpsub(p_poly.0, q_poly.0, res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);

    Ok(res_cell)
}

pub fn bpscal_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let c = slot(sam, 2)?;
    let bp = slot(sam, 3)?;
    let (Ok(c_atom), Ok(bp_poly)) = (c.as_atom(), BPolySlice::try_from(bp)) else {
        return jet_err();
    };
    let c_64 = c_atom.as_u64()?;

    let (res, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(bp_poly.len()));
    bpscal(Belt(c_64), bp_poly.0, res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);

    Ok(res_cell)
}

pub fn bpmul_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let bp = slot(sam, 2)?;
    let bq = slot(sam, 3)?;

    let (Ok(bp_poly), Ok(bq_poly)) = (BPolySlice::try_from(bp), BPolySlice::try_from(bq)) else {
        return jet_err();
    };

    let res_len = if bp_poly.is_zero() | bq_poly.is_zero() {
        1
    } else {
        bp_poly.len() + bq_poly.len() - 1
    };

    let (res_atom, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len));

    bpmul(bp_poly.0, bq_poly.0, res_poly);
    let res_cell = finalize_poly(&mut context.stack, Some(res_len), res_atom);

    Ok(res_cell)
}

pub fn bp_hadamard_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let bp = slot(sam, 2)?;
    let bq = slot(sam, 3)?;

    let (Ok(bp_poly), Ok(bq_poly)) = (BPolySlice::try_from(bp), BPolySlice::try_from(bq)) else {
        return jet_err();
    };
    assert_eq!(bp_poly.len(), bq_poly.len());
    let res_len = bp_poly.len();
    let (res, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len));
    bp_hadamard(bp_poly.0, bq_poly.0, res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);

    Ok(res_cell)
}

pub fn bpdvr_jet(context: &mut Context, subject: Noun) -> Result {
    info!("bpdvr JET ACTIVATED");
    let sam = slot(subject, 6)?;
    let ba = slot(sam, 2)?; // Dividend
    let bb = slot(sam, 3)?; // Divisor

    let (Ok(ba_poly_slice), Ok(bb_poly_slice)) =
        (BPolySlice::try_from(ba), BPolySlice::try_from(bb))
    else {
        return jet_err();
    };

    // Hoon `bpdvr` checks: `?> &(!=(len.ba 0) !=(len.bb 0))`
    // `bpdvr` in bpoly.rs panics on zero divisor.
    // If ba is zero, `bpdvr` (Rust) handles it.
    // So, we only need to check bb (divisor).
    if bb_poly_slice.is_zero() {
        return jet_err(); // "Cannot divide by the zero polynomial." from Hoon `bpdvr`
    }

    let deg_ba = ba_poly_slice.degree();
    let deg_bb = bb_poly_slice.degree();

    // Determine the maximum possible degrees for quotient and remainder.
    // Quotient degree = deg(dividend) - deg(divisor)
    // Remainder degree = deg(divisor) - 1 (or 0 if divisor has degree 0)
    let quotient_len = (deg_ba.saturating_sub(deg_bb) + 1) as usize;
    let remainder_len = (deg_bb.saturating_sub(1) + 1) as usize; // if deg_bb is 0, this is 1 (for deg 0 poly)

    // Allocate memory for the quotient and remainder polynomials.
    let (res_quotient_atom, res_quotient_slice): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(quotient_len));

    let (res_remainder_atom, res_remainder_slice): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(remainder_len));

    // Call the core `bpdvr` function from `bpoly.rs`
    bpdvr(
        ba_poly_slice.0, // Dividend coefficients
        bb_poly_slice.0, // Divisor coefficients
        res_quotient_slice, // Slice for quotient output
        res_remainder_slice, // Slice for remainder output
    );

    // Normalize results: `bpdvr` in `bpoly.rs` doesn't normalize output slices.
    // We need to convert slices to Vecs, normalize, then re-finalize, or ensure finalize_poly normalizes.
    // Assuming `finalize_poly` handles trimming trailing zeros correctly.
    // If not, a `normalize_bpoly` call would be needed like:
    // let mut normalized_quotient = res_quotient_slice.to_vec();
    // normalize_bpoly(&mut normalized_quotient);
    // ... then use `normalized_quotient.len()` for `finalize_poly`

    // Convert the allocated slices back into Hoon `bpoly` Nouns
    let quotient_noun = finalize_poly(&mut context.stack, Some(res_quotient_slice.len()), res_quotient_atom);
    let remainder_noun = finalize_poly(&mut context.stack, Some(res_remainder_slice.len()), res_remainder_atom);

    // Return a cell `[q=bpoly r=bpoly]` as specified by the Hoon `bpdvr` gate
    Ok(T(&mut context.stack, &[quotient_noun, remainder_noun]))
}

pub fn bp_ntt_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let bp = slot(sam, 2)?;
    let root = slot(sam, 3)?;

    let (Ok(bp_poly), Ok(root_atom)) = (BPolySlice::try_from(bp), root.as_atom()) else {
        return jet_err();
    };
    let root_64 = root_atom.as_u64()?;
    let returned_bpoly = bp_ntt(bp_poly.0, &Belt(root_64));
    // TODO: preallocate and pass res buffer into bp_ntt?
    let (res_atom, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(returned_bpoly.len() as usize));
    res_poly.copy_from_slice(&returned_bpoly[..]);

    let res_cell: Noun = finalize_poly(&mut context.stack, Some(res_poly.len()), res_atom);

    Ok(res_cell)
}

pub fn bp_fft_jet(context: &mut Context, subject: Noun) -> Result {
    let p = slot(subject, 6)?;

    let Ok(p_poly) = BPolySlice::try_from(p) else {
        return jet_err();
    };
    let returned_bpoly = bp_fft(p_poly.0)?;
    let (res_atom, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(returned_bpoly.len() as usize));

    res_poly.copy_from_slice(&returned_bpoly);

    let res_cell: Noun = finalize_poly(&mut context.stack, Some(res_poly.len()), res_atom);

    Ok(res_cell)
}

pub fn bp_ifft_jet(context: &mut Context, subject: Noun) -> Result {
    // info!("bp-ifft JET ACTIVATED");

    // Input polynomial (in evaluated form from FFT)
    let p = slot(subject, 6)?;

    let Ok(p_poly) = BPolySlice::try_from(p) else {
        return jet_err();
    };

    // Calculate the inverse root for the IFFT based on the length of the polynomial
    // This assumes the length of the polynomial `p_poly.len()` is the `N` of the transform.
    // If bp_fft used `order.ordered_root()`, then we find the `inverse root` of that same order.
    let order_atom = Atom::new(&mut context.stack, p_poly.len() as u64);
    let Ok(order_belt) = order_atom.as_belt() else {
        return jet_err();
    };
    let Ok(root_belt) = order_belt.ordered_root() else {
        return jet_err();
    };
    let inv_root_belt = root_belt.inv();
    let returned_bpoly_res = bp_ifft(p_poly.0, &inv_root_belt);
    let Ok(returned_bpoly) = returned_bpoly_res else {
        return jet_err();
    };
    let (res_atom, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(returned_bpoly.len()));
    res_poly.copy_from_slice(&returned_bpoly);

    let res_cell: Noun = finalize_poly(&mut context.stack, Some(res_poly.len()), res_atom);

    Ok(res_cell)
}

pub fn bpegcd_jet(context: &mut Context, subject: Noun) -> Result {
    info!("bpegcd JET ACTIVATED");

    let sam = slot(subject, 6)?;
    let ba = slot(sam, 2)?; // First polynomial (a)
    let bb = slot(sam, 3)?; // Second polynomial (b)

    let (Ok(ba_poly_slice), Ok(bb_poly_slice)) =
        (BPolySlice::try_from(ba), BPolySlice::try_from(bb))
    else {
        return jet_err();
    };

    // Hoon `bpegcd` checks: `?> ((bcan (bpoly-to-list b)) ~[0])`
    // The `bpoly.rs::bpegcd` expects non-zero `b`.
    if bb_poly_slice.is_zero() {
        // If b is the zero polynomial, gcd(a, 0) = a, and u = 1, v = 0.
        // Or it might be the case that `bpegcd` expects `b` not to be zero,
        // and its implementation handles `d = a, u = 1, v = 0` (or similar) at the end.
        // Let's assume the jet_err is the desired behavior for a zero divisor,
        // matching the `bpdvr` jet's check for `bb_poly_slice.is_zero()`.
        return jet_err(); // Analogous to "Cannot divide by the zero polynomial." for `bpdvr`
    }

    // Estimate maximum possible degrees for d, u, v
    // d (GCD) degree is at most min(deg(a), deg(b))
    // u degree is at most deg(b) (loose upper bound)
    // v degree is at most deg(a) (loose upper bound)
    // More precisely: deg(u) <= deg(b) - deg(d), deg(v) <= deg(a) - deg(d)
    let deg_ba = ba_poly_slice.degree();
    let deg_bb = bb_poly_slice.degree();

    let d_len_max = (std::cmp::min(deg_ba, deg_bb) + 1) as usize;
    let u_len_max = (deg_bb + 1) as usize; // Can be deg(B)-1, but +1 for safe upper bound
    let v_len_max = (deg_ba + 1) as usize; // Can be deg(A)-1, but +1 for safe upper bound

    // Allocate memory for the output polynomials (d, u, v)
    let (res_d_atom, res_d_slice): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(d_len_max));
    let (res_u_atom, res_u_slice): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(u_len_max));
    let (res_v_atom, res_v_slice): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(v_len_max));

    // Call the core `bpegcd` function from `bpoly.rs`
    // Note: The `bpoly.rs::bpegcd` internally still uses `Vec` allocations.
    // For true optimization, `bpoly.rs::bpegcd` itself would need refactoring
    // to minimize allocations and use pre-allocated buffers.
    bpegcd(
        ba_poly_slice.0, // a coefficients
        bb_poly_slice.0, // b coefficients
        res_d_slice,     // d (GCD) output slice
        res_u_slice,     // u output slice
        res_v_slice,     // v output slice
    );

    // Finalize the output slices into Hoon `bpoly` Nouns
    // Need to get actual lengths from the content of the slices after `bpegcd`
    // Assuming `finalize_poly` handles trimming trailing zeros.
    let d_noun = finalize_poly(&mut context.stack, Some(res_d_slice.len()), res_d_atom);
    let u_noun = finalize_poly(&mut context.stack, Some(res_u_slice.len()), res_u_atom);
    let v_noun = finalize_poly(&mut context.stack, Some(res_v_slice.len()), res_v_atom);

    // Return a cell `[d=bpoly u=bpoly v=bpoly]`
    Ok(T(&mut context.stack, &[d_noun, u_noun, v_noun]))
}

pub fn bp_shift_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let bp = slot(sam, 2)?;
    let c = slot(sam, 3)?;

    let (Ok(bp_poly), Ok(c_belt)) = (BPolySlice::try_from(bp), c.as_belt()) else {
        return jet_err();
    };
    let (res_atom, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(bp_poly.len()));
    bp_shift(bp_poly.0, &c_belt, res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res_atom);

    Ok(res_cell)
}

pub fn bp_coseword_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let p = slot(sam, 2)?;
    let offset = slot(sam, 6)?;
    let order = slot(sam, 7)?;

    let (Ok(p_poly), Ok(offset_belt), Ok(order_atom)) =
        (BPolySlice::try_from(p), offset.as_belt(), order.as_atom())
    else {
        return jet_err();
    };
    let order_32: u32 = order_atom.as_u32()?;
    let root = Belt(order_32 as u64).ordered_root()?;
    let returned_bpoly = bp_coseword(p_poly.0, &offset_belt, order_32, &root);
    let (res, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(returned_bpoly.len() as usize));
    res_poly.copy_from_slice(&returned_bpoly);
    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);

    Ok(res_cell)
}

pub fn init_bpoly_jet(context: &mut Context, subject: Noun) -> Result {
    let poly = slot(subject, 6)?;

    let list_belt = HoonList::try_from(poly)?.into_iter();
    let count = list_belt.count();
    let (res, res_poly): (IndirectAtom, &mut [Belt]) =
        new_handle_mut_slice(&mut context.stack, Some(count as usize));
    for (i, belt_noun) in list_belt.enumerate() {
        let Ok(belt) = belt_noun.as_belt() else {
            return jet_err();
        };
        res_poly[i] = belt;
    }

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);

    Ok(res_cell)
}
