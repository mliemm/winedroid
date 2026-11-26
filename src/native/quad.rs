const FRACTION_BITS: u32 = 52;
const QUAD_FRACTION_BITS: u32 = 112;
const FRACTION_SHIFT: u32 = QUAD_FRACTION_BITS - FRACTION_BITS;
const QUAD_EXPONENT_BIAS: i32 = 16383;
const QUAD_EXPONENT_MASK: u128 = 0x7fff;

pub fn f64_to_quad(value: f64) -> u128 {
    let bits = value.to_bits();
    let sign = u128::from(bits >> 63) << 127;
    let exponent = ((bits >> FRACTION_BITS) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << FRACTION_BITS) - 1);
    if exponent == 0x7ff {
        return sign
            | (QUAD_EXPONENT_MASK << QUAD_FRACTION_BITS)
            | (u128::from(fraction) << FRACTION_SHIFT);
    }
    if exponent == 0 {
        if fraction == 0 {
            return sign;
        }
        let shift = fraction.leading_zeros() as i32 - 11;
        let significand = fraction << shift;
        let biased = (QUAD_EXPONENT_BIAS - 1022 - shift) as u128;
        return sign
            | (biased << QUAD_FRACTION_BITS)
            | (u128::from(significand & ((1u64 << FRACTION_BITS) - 1)) << FRACTION_SHIFT);
    }
    let biased = (exponent - 1023 + QUAD_EXPONENT_BIAS) as u128;
    sign | (biased << QUAD_FRACTION_BITS) | (u128::from(fraction) << FRACTION_SHIFT)
}

pub fn quad_to_f64_exact(value: u128) -> Option<f64> {
    let sign = ((value >> 127) as u64) << 63;
    let exponent = ((value >> QUAD_FRACTION_BITS) & QUAD_EXPONENT_MASK) as i32;
    let fraction = value & ((1u128 << QUAD_FRACTION_BITS) - 1);
    if exponent == QUAD_EXPONENT_MASK as i32 {
        if fraction & ((1u128 << FRACTION_SHIFT) - 1) != 0 {
            return None;
        }
        return Some(f64::from_bits(
            sign | (0x7ffu64 << FRACTION_BITS) | (fraction >> FRACTION_SHIFT) as u64,
        ));
    }
    if exponent == 0 {
        return (fraction == 0).then(|| f64::from_bits(sign));
    }
    let unbiased = exponent - QUAD_EXPONENT_BIAS;
    let significand = (1u128 << QUAD_FRACTION_BITS) | fraction;
    if unbiased > 1023 {
        return None;
    }
    if unbiased >= -1022 {
        if fraction & ((1u128 << FRACTION_SHIFT) - 1) != 0 {
            return None;
        }
        let biased = (unbiased + 1023) as u64;
        return Some(f64::from_bits(
            sign | (biased << FRACTION_BITS) | (fraction >> FRACTION_SHIFT) as u64,
        ));
    }
    let drop = FRACTION_SHIFT as i32 + (-1022 - unbiased);
    if drop >= 128 || significand & ((1u128 << drop) - 1) != 0 {
        return None;
    }
    Some(f64::from_bits(sign | (significand >> drop) as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee_binary128_encodes_one_and_minus_two_with_the_documented_fields() {
        assert_eq!(f64_to_quad(1.0), 0x3fffu128 << 112);
        assert_eq!(f64_to_quad(-2.0), (1u128 << 127) | (0x4000u128 << 112));
        assert_eq!(f64_to_quad(0.0), 0);
        assert_eq!(f64_to_quad(f64::INFINITY), 0x7fffu128 << 112);
    }

    #[test]
    fn ieee_binary128_roundtrips_every_f64_class_exactly() {
        let subnormal = f64::from_bits(0x0000_0000_0000_0005);
        for value in [
            1.5,
            -0.0,
            0.0,
            6.25e300,
            -3.5e-300,
            subnormal,
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::NEG_INFINITY,
        ] {
            let back = quad_to_f64_exact(f64_to_quad(value)).unwrap();
            assert_eq!(back.to_bits(), value.to_bits(), "{value:e}");
        }
        assert!(quad_to_f64_exact(f64_to_quad(f64::NAN)).unwrap().is_nan());
    }

    #[test]
    fn ieee_binary128_values_with_more_than_f64_precision_are_refused() {
        let one_plus_tiny = (0x3fffu128 << 112) | 1;
        assert_eq!(quad_to_f64_exact(one_plus_tiny), None);
        assert_eq!(quad_to_f64_exact(0x7ffeu128 << 112), None);
    }
}
