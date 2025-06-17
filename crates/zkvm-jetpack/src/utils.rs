// Utility functions and commonly used re-exports

use nockvm::interpreter::Context;
use nockvm::noun::{Atom, IndirectAtom, DIRECT_MAX, Noun, D, T};
pub use tracing::{debug, trace};
use nockvm::mem::NockStack;
use nockvm::jets::JetErr;

use crate::form::{Belt};

use bitvec::prelude::{BitSlice, Lsb0};
use ibig::UBig;

// tests whether a felt atom has the leading 1. we cannot actually test
// Felt, because it doesn't include the leading 1.
pub fn felt_atom_is_valid(felt_atom: IndirectAtom) -> bool {
    let dat_ptr = felt_atom.data_pointer();
    unsafe { *(dat_ptr.add(3)) == 0x1 }
}

pub fn vec_to_hoon_list(context: &mut Context, vec: &[u64]) -> Noun {
    let mut list = D(0);
    for e in vec.iter().rev() {
        let n = Atom::new(&mut context.stack, *e).as_noun();
        list = T(&mut context.stack, &[n, list]);
    }
    list
}

pub fn bitslice_to_u128(bits: &BitSlice<u64, Lsb0>) -> u128 {
    bits.iter().by_vals().enumerate().fold(
        0u128,
        |acc, (i, bit)| {
            if bit {
                acc | (1u128 << i)
            } else {
                acc
            }
        },
    )
}

pub fn fits_in_u128(bits: &BitSlice<u64, Lsb0>) -> bool {
    bits.iter()
        .by_vals()
        .enumerate()
        .rfind(|&(_, bit)| bit)
        .map_or(true, |(i, _)| i <= 127)
}

pub fn belt_as_noun(stack: &mut NockStack, res: Belt) -> Noun {
    u128_as_noun(stack, res.0 as u128)
}

pub fn u128_as_noun(stack: &mut NockStack, res: u128) -> Noun {
    if res < DIRECT_MAX as u128 {
        D(res as u64)
    } else {
        let res_big = UBig::from(res);
        Atom::from_ubig(stack, &res_big).as_noun()
    }
}

pub fn hoon_list_to_vecbelt(list: Noun) -> Result<Vec<Belt>, JetErr> {
    let mut input_iterate = list;
    let mut input_vec: Vec<Belt> = Vec::new();
    while unsafe { !input_iterate.raw_equals(&D(0)) } {
        let input_cell = input_iterate.as_cell()?;
        let head_belt = Belt(input_cell.head().as_atom()?.as_u64()?);
        input_vec.push(head_belt);
        input_iterate = input_cell.tail();
    }

    Ok(input_vec)
}