use nockvm::interpreter::Context;
use nockvm::jets::util::slot;
use nockvm::jets::Result;
use nockvm::noun::{IndirectAtom, Noun, D, T};

use crate::form::math::fext::*;
use crate::form::poly::*;
use crate::hand::handle::*;
use crate::jets::utils::jet_err;
use crate::noun::noun_ext::NounExt;

// Helper function to convert fpoly to list (for debugging/testing)
pub fn fpoly_to_list_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    fpoly_to_list(context, sam)
}

pub fn fpoly_to_list(context: &mut Context, sam: Noun) -> Result {
    let Ok(sam_fpoly) = FPolySlice::try_from(sam) else {
        return jet_err();
    };

    // Empty list is a null atom
    let mut res_list = D(0);
    let len = sam_fpoly.len();

    if len == 0 {
        return Ok(res_list);
    }

    for i in (0..len).rev() {
        // Convert Felt to IndirectAtom (3 u64s)
        let felt = &sam_fpoly.data()[i];
        let mut bytes = Vec::with_capacity(24);
        bytes.extend_from_slice(&felt.0[0].0.to_le_bytes());
        bytes.extend_from_slice(&felt.0[1].0.to_le_bytes());
        bytes.extend_from_slice(&felt.0[2].0.to_le_bytes());
        
        let res_atom = unsafe { IndirectAtom::new_raw_bytes(&mut context.stack, bytes.len(), bytes.as_ptr()) };
        res_list = T(&mut context.stack, &[res_atom.as_noun(), res_list]);
    }

    Ok(res_list)
}

// fp_add_jet: Field polynomial addition
pub fn fp_add_jet(context: &mut Context, subject: Noun) -> Result {
    // Debug logging to verify jet is being invoked
    // eprintln!("[JET] fp_add_jet invoked!");
    
    let sam = slot(subject, 6)?;
    let fp = slot(sam, 2)?;
    let fq = slot(sam, 3)?;

    let (Ok(fp_poly), Ok(fq_poly)) = (FPolySlice::try_from(fp), FPolySlice::try_from(fq)) else {
        return jet_err();
    };

    let res_len = std::cmp::max(fp_poly.len(), fq_poly.len());
    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len));
    
    fpadd_poly(fp_poly.data(), fq_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// fp_neg_jet: Field polynomial negation
pub fn fp_neg_jet(context: &mut Context, subject: Noun) -> Result {
    let fp = slot(subject, 6)?;

    let Ok(fp_poly) = FPolySlice::try_from(fp) else {
        return jet_err();
    };

    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(fp_poly.len()));
    
    fpneg_poly(fp_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// fp_sub_jet: Field polynomial subtraction
pub fn fp_sub_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let p = slot(sam, 2)?;
    let q = slot(sam, 3)?;

    let (Ok(p_poly), Ok(q_poly)) = (FPolySlice::try_from(p), FPolySlice::try_from(q)) else {
        return jet_err();
    };

    let res_len = std::cmp::max(p_poly.len(), q_poly.len());
    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len));
    
    fpsub_poly(p_poly.data(), q_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// fp_scal_jet: Scale field polynomial by a field element
pub fn fp_scal_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let c = slot(sam, 2)?;
    let fp = slot(sam, 3)?;
    
    let Ok(fp_poly) = FPolySlice::try_from(fp) else {
        return jet_err();
    };

    // Extract the Felt scalar from c
    let Ok(c_felt) = c.as_felt() else {
        return jet_err();
    };

    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(fp_poly.len()));
    
    fpscal_poly(c_felt, fp_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// fp_mul_jet: Field polynomial multiplication
pub fn fp_mul_jet(context: &mut Context, subject: Noun) -> Result {
    // Debug logging to verify jet is being invoked
    // eprintln!("[JET] fp_mul_jet invoked!");
    
    let sam = slot(subject, 6)?;
    let fp = slot(sam, 2)?;
    let fq = slot(sam, 3)?;

    let (Ok(fp_poly), Ok(fq_poly)) = (FPolySlice::try_from(fp), FPolySlice::try_from(fq)) else {
        return jet_err();
    };

    // Result length is sum of degrees + 1
    let res_len = if fp_poly.len() == 0 || fq_poly.len() == 0 {
        0
    } else {
        fp_poly.len() + fq_poly.len() - 1
    };

    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len));
    
    fpmul_poly(fp_poly.data(), fq_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// fp_eval_jet: Evaluate polynomial at a point using Horner's method
pub fn fp_eval_jet(context: &mut Context, subject: Noun) -> Result {
    // Debug logging to verify jet is being invoked
    // eprintln!("[JET] fp_eval_jet invoked!");
    
    let sam = slot(subject, 6)?;
    let fp = slot(sam, 2)?;
    let x = slot(sam, 3)?;

    let Ok(fp_poly) = FPolySlice::try_from(fp) else {
        return jet_err();
    };

    let Ok(x_felt_ref) = x.as_felt() else {
        return jet_err();
    };
    
    let result = fpeval_poly(fp_poly.data(), x_felt_ref);
    
    // Convert Felt result to Atom
    let mut bytes = Vec::with_capacity(24);
    bytes.extend_from_slice(&result.0[0].0.to_le_bytes());
    bytes.extend_from_slice(&result.0[1].0.to_le_bytes());
    bytes.extend_from_slice(&result.0[2].0.to_le_bytes());
    
    let res_atom = unsafe { IndirectAtom::new_raw_bytes(&mut context.stack, bytes.len(), bytes.as_ptr()) };
    Ok(res_atom.as_noun())
}

// fp_fft_jet: Fast Fourier Transform for field polynomials
pub fn fp_fft_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    
    let Ok(fp_poly) = FPolySlice::try_from(sam) else {
        return jet_err();
    };

    let len = fp_poly.len();
    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(len));
    
    fp_fft_poly(fp_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// fp_ifft_jet: Inverse Fast Fourier Transform
pub fn fp_ifft_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    
    let Ok(fp_poly) = FPolySlice::try_from(sam) else {
        return jet_err();
    };

    let len = fp_poly.len();
    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(len));
    
    fp_ifft_poly(fp_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// FFT using Number Theoretic Transform (NTT) algorithm - matches Hoon's fp-ntt
fn fp_fft_poly(p: &[Felt], res: &mut [Felt]) {
    let n = p.len();
    
    // Must be power of 2
    assert!(n & (n - 1) == 0, "FFT requires power-of-2 length");
    
    // Base case: if length is 1, just copy
    if n == 1 {
        res[0] = p[0];
        return;
    }
    
    let half = n / 2;
    let log_n = n.trailing_zeros() as usize;
    let root = get_root_of_unity(log_n);
    
    // Separate even and odd indices
    let mut evens = vec![Felt::zero(); half];
    let mut odds = vec![Felt::zero(); half];
    
    for i in 0..n {
        if i % 2 == 0 {
            evens[i / 2] = p[i];
        } else {
            odds[i / 2] = p[i];
        }
    }
    
    // Recursively compute FFT of evens and odds
    let mut evens_fft = vec![Felt::zero(); half];
    let mut odds_fft = vec![Felt::zero(); half];
    
    // Square the root for recursive calls
    let mut root_squared = Felt::zero();
    fmul(&root, &root, &mut root_squared);
    
    // Recursive FFT on halves
    fp_fft_recursive(&evens, &mut evens_fft, &root_squared);
    fp_fft_recursive(&odds, &mut odds_fft, &root_squared);
    
    // Combine results: res[i] = evens_fft[i % half] + root^i * odds_fft[i % half]
    for i in 0..n {
        let mut root_power = fpow_(&root, i as u64);
        let mut term = Felt::zero();
        fmul(&root_power, &odds_fft[i % half], &mut term);
        fadd(&evens_fft[i % half], &term, &mut res[i]);
    }
}

// Recursive helper for FFT
fn fp_fft_recursive(p: &[Felt], res: &mut [Felt], root: &Felt) {
    let n = p.len();
    
    if n == 1 {
        res[0] = p[0];
        return;
    }
    
    let half = n / 2;
    
    // Separate even and odd indices
    let mut evens = vec![Felt::zero(); half];
    let mut odds = vec![Felt::zero(); half];
    
    for i in 0..n {
        if i % 2 == 0 {
            evens[i / 2] = p[i];
        } else {
            odds[i / 2] = p[i];
        }
    }
    
    // Square the root for recursive calls
    let mut root_squared = Felt::zero();
    fmul(root, root, &mut root_squared);
    
    // Recursive FFT on halves
    let mut evens_fft = vec![Felt::zero(); half];
    let mut odds_fft = vec![Felt::zero(); half];
    
    fp_fft_recursive(&evens, &mut evens_fft, &root_squared);
    fp_fft_recursive(&odds, &mut odds_fft, &root_squared);
    
    // Combine results
    for i in 0..n {
        let mut root_power = fpow_(root, i as u64);
        let mut term = Felt::zero();
        fmul(&root_power, &odds_fft[i % half], &mut term);
        fadd(&evens_fft[i % half], &term, &mut res[i]);
    }
}

// Inverse FFT implementation - matches Hoon's fp-ifft
fn fp_ifft_poly(p: &[Felt], res: &mut [Felt]) {
    let n = p.len();
    
    // Must be power of 2
    assert!(n & (n - 1) == 0, "IFFT requires power-of-2 length");
    
    // Get root of unity and invert it
    let log_n = n.trailing_zeros() as usize;
    let root = get_root_of_unity(log_n);
    let mut inv_root = Felt::zero();
    finv(&root, &mut inv_root);
    
    // Run FFT with inverse root
    fp_fft_recursive(p, res, &inv_root);
    
    // Scale by 1/n
    let n_felt = Felt::from([Belt(n as u64), Belt(0), Belt(0)]);
    let mut inv_n = Felt::zero();
    finv(&n_felt, &mut inv_n);
    
    for i in 0..n {
        let temp = res[i];
        fmul(&temp, &inv_n, &mut res[i]);
    }
}

// interpolate_jet: Lagrange interpolation
pub fn interpolate_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let domain = slot(sam, 2)?;
    let values = slot(sam, 3)?;

    let (Ok(domain_poly), Ok(values_poly)) = 
        (FPolySlice::try_from(domain), FPolySlice::try_from(values)) else {
        return jet_err();
    };

    if domain_poly.len() != values_poly.len() {
        return jet_err();
    }

    let len = domain_poly.len();
    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(len));
    
    interpolate_poly(domain_poly.data(), values_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// fpcompose_jet: Polynomial composition P(Q(X))
pub fn fpcompose_jet(context: &mut Context, subject: Noun) -> Result {
    let sam = slot(subject, 6)?;
    let p = slot(sam, 2)?;
    let q = slot(sam, 3)?;

    let (Ok(p_poly), Ok(q_poly)) = (FPolySlice::try_from(p), FPolySlice::try_from(q)) else {
        return jet_err();
    };

    // Result degree is deg(p) * deg(q)
    let res_len = if p_poly.len() == 0 || q_poly.len() == 0 {
        0
    } else {
        (p_poly.len() - 1) * (q_poly.len() - 1) + 1
    };

    let (res, res_poly): (IndirectAtom, &mut [Felt]) =
        new_handle_mut_slice(&mut context.stack, Some(res_len));
    
    fpcompose_poly(p_poly.data(), q_poly.data(), res_poly);

    let res_cell = finalize_poly(&mut context.stack, Some(res_poly.len()), res);
    Ok(res_cell)
}

// ============================================================================
// Field polynomial math operations
// ============================================================================

// Field polynomial addition
fn fpadd_poly(p: &[Felt], q: &[Felt], res: &mut [Felt]) {
    let lp = p.len();
    let lq = q.len();
    let m = std::cmp::max(lp, lq);

    // Initialize result to zero
    for i in 0..m {
        res[i] = Felt::zero();
    }

    // Add p
    for i in 0..lp {
        let temp = res[i];
        fadd(&p[i], &temp, &mut res[i]);
    }

    // Add q
    for i in 0..lq {
        let temp = res[i];
        fadd(&q[i], &temp, &mut res[i]);
    }
}

// Field polynomial negation
fn fpneg_poly(p: &[Felt], res: &mut [Felt]) {
    for i in 0..p.len() {
        fneg(&p[i], &mut res[i]);
    }
}

// Field polynomial subtraction
fn fpsub_poly(p: &[Felt], q: &[Felt], res: &mut [Felt]) {
    let lp = p.len();
    let lq = q.len();
    let m = std::cmp::max(lp, lq);

    // Initialize result to zero
    for i in 0..m {
        res[i] = Felt::zero();
    }

    // Add p
    for i in 0..lp {
        let temp = res[i];
        fadd(&p[i], &temp, &mut res[i]);
    }

    // Subtract q
    for i in 0..lq {
        let mut neg_q = Felt::zero();
        fneg(&q[i], &mut neg_q);
        let temp = res[i];
        fadd(&neg_q, &temp, &mut res[i]);
    }
}

// Scale polynomial by field element
fn fpscal_poly(c: &Felt, p: &[Felt], res: &mut [Felt]) {
    for i in 0..p.len() {
        fmul(c, &p[i], &mut res[i]);
    }
}

// Field polynomial multiplication (naive O(n²) algorithm)
fn fpmul_poly(p: &[Felt], q: &[Felt], res: &mut [Felt]) {
    let lp = p.len();
    let lq = q.len();

    if lp == 0 || lq == 0 {
        return;
    }

    // Initialize result to zero
    for i in 0..res.len() {
        res[i] = Felt::zero();
    }

    // Multiply each term of p with each term of q
    for i in 0..lp {
        for j in 0..lq {
            let mut prod = Felt::zero();
            fmul(&p[i], &q[j], &mut prod);
            let temp = res[i + j];
            fadd(&prod, &temp, &mut res[i + j]);
        }
    }
}

// Evaluate polynomial at a point using Horner's method
fn fpeval_poly(p: &[Felt], x: &Felt) -> Felt {
    if p.is_empty() {
        return Felt::zero();
    }

    let mut result = p[p.len() - 1];
    
    for i in (0..p.len() - 1).rev() {
        let mut temp = Felt::zero();
        fmul(&result, x, &mut temp);
        fadd(&temp, &p[i], &mut result);
    }

    result
}

// Helper function to get root of unity for given log size
fn get_root_of_unity(log_n: usize) -> Felt {
    // These are the same precomputed roots from the Hoon code
    const ROOTS: &[u64] = &[
        0x0000000000000001, 0xffffffff00000000, 0x0001000000000000, 0xfffffffeff000001,
        0xefffffff00000001, 0x00003fffffffc000, 0x0000008000000000, 0xf80007ff08000001,
        0xbf79143ce60ca966, 0x1905d02a5c411f4e, 0x9d8f2ad78bfed972, 0x0653b4801da1c8cf,
        0xf2c35199959dfcb6, 0x1544ef2335d17997, 0xe0ee099310bba1e2, 0xf6b2cffe2306baac,
        0x54df9630bf79450e, 0xabd0a6e8aa3d8a0e, 0x81281a7b05f9beac, 0xfbd41c6b8caa3302,
        0x30ba2ecd5e93e76d, 0xf502aef532322654, 0x4b2a18ade67246b5, 0xea9d5a1336fbc98b,
        0x86cdcc31c307e171, 0x4bbaf5976ecfefd8, 0xed41d05b78d6e286, 0x10d78dd8915a171d,
        0x59049500004a4485, 0xdfa8c93ba46d2666, 0x7e9bd009b86a0845, 0x400a7f755588e659,
        0x185629dcda58878c,
    ];
    
    assert!(log_n < ROOTS.len(), "FFT size too large");
    Felt::from([Belt(ROOTS[log_n]), Belt(0), Belt(0)])
}

// Lagrange interpolation to find polynomial through given points
fn interpolate_poly(domain: &[Felt], values: &[Felt], res: &mut [Felt]) {
    let n = domain.len();
    
    // Initialize result polynomial to zero
    for i in 0..res.len() {
        res[i] = Felt::zero();
    }

    // For each data point (domain[i], values[i])
    for i in 0..n {
        // Compute the Lagrange basis polynomial L_i(x)
        // L_i(x) = product_{j!=i} (x - domain[j]) / (domain[i] - domain[j])
        
        // Start with the constant polynomial values[i] / denominator
        // We'll compute denominator first
        let mut denom = Felt::one();
        for j in 0..n {
            if i != j {
                let mut diff = Felt::zero();
                fsub(&domain[i], &domain[j], &mut diff);
                let mut new_denom = Felt::zero();
                fmul(&denom, &diff, &mut new_denom);
                denom = new_denom;
            }
        }
        
        // Scale factor = values[i] / denom
        let mut scale = Felt::zero();
        fdiv(&values[i], &denom, &mut scale);
        
        // Now build the numerator polynomial product_{j!=i} (x - domain[j])
        // We'll use a temporary polynomial and multiply iteratively
        let mut basis = vec![Felt::zero(); n];
        basis[0] = Felt::one(); // Start with constant polynomial 1
        let mut basis_deg = 1; // Current degree + 1
        
        for j in 0..n {
            if i != j {
                // Multiply basis by (x - domain[j])
                // This means: new_basis = x * basis - domain[j] * basis
                let mut new_basis = vec![Felt::zero(); basis_deg + 1];
                
                // x * basis part (shift coefficients up)
                for k in 0..basis_deg {
                    new_basis[k + 1] = basis[k];
                }
                
                // - domain[j] * basis part
                for k in 0..basis_deg {
                    let mut prod = Felt::zero();
                    fmul(&domain[j], &basis[k], &mut prod);
                    let mut diff = Felt::zero();
                    fsub(&new_basis[k], &prod, &mut diff);
                    new_basis[k] = diff;
                }
                
                basis = new_basis;
                basis_deg += 1;
            }
        }
        
        // Now add scale * basis to result
        for k in 0..basis_deg.min(res.len()) {
            let mut term = Felt::zero();
            fmul(&scale, &basis[k], &mut term);
            let temp = res[k];
            fadd(&temp, &term, &mut res[k]);
        }
    }
}

// Polynomial composition P(Q(X)) (basic implementation)
fn fpcompose_poly(p: &[Felt], q: &[Felt], res: &mut [Felt]) {
    if p.is_empty() || q.is_empty() {
        return;
    }

    // Initialize result to zero
    for i in 0..res.len() {
        res[i] = Felt::zero();
    }

    // Start with p[0]
    res[0] = p[0];

    // Compute powers of Q and accumulate
    let mut q_power = vec![Felt::one()]; // Q^0 = 1
    
    for i in 1..p.len() {
        // Multiply q_power by q to get next power
        let new_len = q_power.len() + q.len() - 1;
        let mut new_q_power = vec![Felt::zero(); new_len];
        
        for j in 0..q_power.len() {
            for k in 0..q.len() {
                let mut prod = Felt::zero();
                fmul(&q_power[j], &q[k], &mut prod);
                let temp = new_q_power[j + k];
                fadd(&prod, &temp, &mut new_q_power[j + k]);
            }
        }
        
        q_power = new_q_power;
        
        // Add p[i] * Q^i to result
        for j in 0..std::cmp::min(q_power.len(), res.len()) {
            let mut term = Felt::zero();
            fmul(&p[i], &q_power[j], &mut term);
            let temp = res[j];
            fadd(&temp, &term, &mut res[j]);
        }
    }
}