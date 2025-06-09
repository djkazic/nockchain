use bs58;
use ibig::{ubig, UBig};
use nockapp::NockAppError;
use nockvm::noun::Noun;
//TODO all this stuff would be useful as jets, which mostly just requires
//using the Atom::as_ubig with the NockStack instead of ibig's heap version
// which we use to avoid having a NockStack sitting around.

// Goldilocks prime
const P: u64 = 0xffffffff00000001;

/// Tries to convert a Noun to a Base58 string by extracting a 5-tuple, converting it to a decimal, and then to Base58.
///
/// # Arguments
/// * `noun` - The Noun to convert, expected to be a 5-tuple tip5 hash
///
/// # Returns
/// The Noun as a Base58 string
pub fn tip5_hash_to_base58(noun: Noun) -> Result<String, NockAppError> {
    let tuple = extract_5_tuple(noun)?;
    let decimal_value = base_p_to_decimal(tuple)?;
    let base58_string = ubig_to_base58(decimal_value);

    Ok(base58_string)
}

pub fn base_p_to_decimal(hash: Vec<Noun>) -> Result<UBig, NockAppError> {
    let prime_ubig = UBig::from(P);
    let mut result = ubig!(0);

    for (i, noun) in hash.iter().enumerate() {
        // Convert Noun to Atom and then to UBig
        let atom = noun.as_atom()?.as_u64()?;

        // Add the value * P^i to the result
        result += UBig::from(atom) * prime_ubig.pow(i);
    }

    Ok(result)
}

/// Converts a UBig to a Base58 string.
pub fn ubig_to_base58(value: UBig) -> String {
    let bytes = value.to_be_bytes();
    bs58::encode(bytes).into_string()
}

/// Extracts a 5-tuple from a cell, returning the elements as a Vec
pub fn extract_5_tuple(tuple_cell: Noun) -> Result<Vec<Noun>, NockAppError> {
    let mut elements = Vec::with_capacity(5);
    let mut current = tuple_cell;

    // Extract the first 4 elements
    for _ in 0..4 {
        let cell = current.as_cell()?;
        let head = cell.head();
        // Verify that the element is an atom
        head.as_atom()?;
        elements.push(head);
        current = cell.tail();
    }

    // The 5th element is the final item
    // Verify that the last element is an atom
    current.as_atom()?;
    elements.push(current);

    Ok(elements)
}

#[cfg(test)]
mod tests {
    use nockapp::noun::slab::NounSlab;
    use nockvm::noun::{D, T};

    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)] // ibig has a memory leak so miri fails this test
    fn test_tip5_hash_to_base58() {
        use nockvm::noun::Atom;
        // Create a NounSlab to use as an allocator
        let mut slab = NounSlab::new();

        // Test case 1: Simple tuple [1 2 3 4 5]
        let tuple1 = T(&mut slab, &[D(1), D(2), D(3), D(4), D(5)]);
        let expected1 = "2V9arU36gvtaofWmNowewoj9u7gbNA2qsJZEQ3WPky5mQ";
        let result1 = tip5_hash_to_base58(tuple1).unwrap_or_else(|_| {
            panic!(
                "Called `expect()` at {}:{} (git sha: {})",
                file!(),
                line!(),
                option_env!("GIT_SHA").unwrap_or("unknown")
            )
        });
        assert_eq!(result1, expected1);

        // Test case 2: Complex values
        let a1 = Atom::new(&mut slab, 0x6ef99e5f3447ffda);
        let a2 = Atom::new(&mut slab, 0xdf94122d1a98ec99);
        let a3 = Atom::new(&mut slab, 0xcbf1918337a0e197);
        let a4 = Atom::new(&mut slab, 0x6cda1112891244ce);
        let a5 = Atom::new(&mut slab, 0x6e420b8a615508d4);

        let tuple2 = T(
            &mut slab,
            &[a1.as_noun(), a2.as_noun(), a3.as_noun(), a4.as_noun(), a5.as_noun()],
        );
        let expected2 = "6UkUko9WTwwR6VVRXwPQpUy5pswdvNtoyHspY5n9nLVnBxzAgEyMwPR";
        let result2 = tip5_hash_to_base58(tuple2).unwrap_or_else(|err| {
            panic!(
                "Panicked with {err:?} at {}:{} (git sha: {:?})",
                file!(),
                line!(),
                option_env!("GIT_SHA")
            )
        });
        assert_eq!(result2, expected2);
    }
}
