use nockvm::interpreter::{Context, Error, Mote};
use nockvm::jets::JetErr;
use nockvm::jets::JetErr::*;
use nockvm::noun::{Noun, Atom, T, D};
use crate::form::poly::Belt;

use crate::form::math::FieldError;

pub fn jet_err<T>() -> Result<T, JetErr> {
    Err(Fail(Error::Deterministic(Mote::Exit, D(0))))
}

impl From<FieldError> for JetErr {
    fn from(e: FieldError) -> Self {
        match e {
            FieldError::OrderedRootError => Fail(Error::Deterministic(Mote::Exit, D(0))),
        }
    }
}

// Convert a Hoon list of base‐field atoms into a Vec<Belt>.
pub fn noun_to_vec_belt(list: Noun) -> Result<Vec<Belt>, JetErr> {
    if list.is_atom() {
        return jet_err();
    }
    let mut out = Vec::new();
    let mut cur = list;
    while cur.is_cell() {
        let cell = cur.as_cell()?;
        let v    = cell.head().as_atom()?.as_u64()?;
        out.push(Belt(v));
        cur = cell.tail();
    }
    Ok(out)
}

// Convert a Rust Vec<Belt> back into a Hoon list.
pub fn vec_belt_to_noun(ctx: &mut Context, v: &[Belt]) -> Noun {
    let mut list = D(0);
    for &e in v.iter().rev() {
        let atom = Atom::new(&mut ctx.stack, e.0).as_noun();
        list = T(&mut ctx.stack, &[atom, list]);
    }
    list
}