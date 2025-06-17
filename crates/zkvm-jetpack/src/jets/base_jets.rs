use nockvm::interpreter::Context;
use nockvm::jets::util::{bite, slot};
use nockvm::jets::bits::util::rip;
use nockvm::jets::Result;
use nockvm::mem::NockStack;
use nockvm::noun::{Atom, Noun, D, NO, T, YES};
use tracing::debug;

use crate::form::math::base::*;
use crate::form::poly::*;
use crate::jets::utils::*;

// base field jets
//
// When possible, all these functions do is get the sample from the subject,
// convert them into the appropriate datatypes, allocate space for a result,
// hand off the actual business logic elsewhere, and then return the result.
//
// In some cases, like bpmul_jet, this can result in a little more work being
// done than strictly necessary. We could, e.g., check that a polynomial is
// zero and then shortcircuit calling bpmul by just returning zero. Instead,
// we allocate space for a polynomial of sufficient size without checking
// whether either is zero, and then bpmul does the zero check. While this is
// inefficient, it makes division of labor clear.

pub fn badd_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let a = slot(sam, 2)?;
    let b = slot(sam, 3)?;

    let (Ok(a_atom), Ok(b_atom)) = (a.as_atom(), b.as_atom()) else {
        debug!("a or b was not an atom");
        return jet_err();
    };
    let (a_belt, b_belt): (Belt, Belt) = (a_atom.as_u64()?.into(), b_atom.as_u64()?.into());
    Ok(Atom::new(&mut context.stack, (a_belt + b_belt).into()).as_noun())
}

pub fn bsub_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let a = slot(sam, 2)?;
    let b = slot(sam, 3)?;

    let (Ok(a_atom), Ok(b_atom)) = (a.as_atom(), b.as_atom()) else {
        debug!("a or b was not an atom");
        return jet_err();
    };
    let (a_belt, b_belt): (Belt, Belt) = (a_atom.as_u64()?.into(), b_atom.as_u64()?.into());

    Ok(Atom::new(&mut context.stack, (a_belt - b_belt).into()).as_noun())
}

pub fn bneg_jet(context: &mut Context, subject: Noun) -> Result {
    let a = slot(subject, 6)?;
    let Ok(a_atom) = a.as_atom() else {
        debug!("a was not an atom");
        return jet_err();
    };
    let a_belt: Belt = a_atom.as_u64()?.into();

    Ok(Atom::new(&mut context.stack, (-a_belt).into()).as_noun())
}

pub fn bmul_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let a = slot(sam, 2)?;
    let b = slot(sam, 3)?;

    let (Ok(a_atom), Ok(b_atom)) = (a.as_atom(), b.as_atom()) else {
        debug!("a or b was not an atom");
        return jet_err();
    };
    let (a_belt, b_belt): (Belt, Belt) = (a_atom.as_u64()?.into(), b_atom.as_u64()?.into());

    Ok(Atom::new(&mut context.stack, (a_belt * b_belt).into()).as_noun())
}

pub fn ordered_root_jet(context: &mut Context, subject: Noun) -> Result {
    let n = slot(subject, 6)?;

    let Ok(n_atom) = n.as_atom() else {
        debug!("n was not an atom");
        return jet_err();
    };
    let n_u64 = Belt(n_atom.as_u64()?);
    // TODO: clean this up
    let res_atom = Atom::new(&mut context.stack, n_u64.ordered_root()?.into());
    Ok(res_atom.as_noun())
}

pub fn bpow_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let x = slot(sam, 2)?;
    let n = slot(sam, 3)?;

    let (Ok(x_atom), Ok(n_atom)) = (x.as_atom(), n.as_atom()) else {
        debug!("x or n was not an atom");
        return jet_err();
    };
    let (x_belt, n_belt) = (x_atom.as_u64()?, n_atom.as_u64()?);

    Ok(Atom::new(&mut context.stack, bpow(x_belt, n_belt)).as_noun())
}

pub fn rip_correct_jet(context: &mut Context, subject: Noun) -> Result {
    let stack = &mut context.stack;
    let sam = slot(subject, 6)?;
    let a_noun = slot(sam, 2)?;
    let b_noun = slot(sam, 3)?;

    let b = b_noun.as_atom()?;
    let (bloq, step) = bite(a_noun)?;
    rip_correct(stack, bloq, step, b)
}

pub fn rip_correct(stack: &mut NockStack, bloq: usize, step: usize, b: Atom) -> Result {
    if b.is_direct() && b.as_u64()? == 0 {
        return Ok(T(stack, &[D(0), D(0)]));
    }
    rip(stack, bloq, step, b)
}

pub fn levy_based(a_noun: Noun) -> bool {
    let mut list = a_noun;
    loop {
        if unsafe { list.raw_equals(&D(0)) } {
            return true;
        }
        let cell = list.as_cell().expect("cell not found");
        let based_res = based_one(cell.head());
        if !based_res { return false; }

        list = cell.tail();
    }
}

pub fn based_one_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    if based_one(sam) { Ok(YES) } else { Ok(NO) }
}

fn based_one(a_noun: Noun) -> bool {
    let a = a_noun.as_atom().expect("a not atom");
    let a_u64 = a.as_u64().expect("a not u64");
    a_u64 < PRIME
}
