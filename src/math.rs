use bigdecimal::{BigDecimal as D, RoundingMode};
use num_traits::{One, ToPrimitive, Zero};
use std::{num::NonZeroU64, str::FromStr, sync::LazyLock};

pub type Number = D;
static MAX_VALUE: LazyLock<D> = LazyLock::new(|| dec("1e32"));
static EPSILON: LazyLock<D> = LazyLock::new(|| dec("1e-62"));
static HALF: LazyLock<D> = LazyLock::new(|| dec("0.5"));
static THREE_QUARTERS: LazyLock<D> = LazyLock::new(|| dec("0.75"));
static ONE_AND_HALF: LazyLock<D> = LazyLock::new(|| dec("1.5"));
static LN_TWO: LazyLock<D> =
    LazyLock::new(|| dec("0.693147180559945309417232121458176568075500134360255254120680009493"));
static TAU: LazyLock<D> =
    LazyLock::new(|| dec("6.283185307179586476925286766559005768394338798750211641949889184615"));
static LN_TEN: LazyLock<D> = LazyLock::new(|| logarithm(&D::from(10)).expect("ln(10) is defined"));
pub fn dec(s: &str) -> D {
    D::from_str(s).expect("decimal constant")
}
pub fn rounded(v: D) -> D {
    v.with_precision_round(NonZeroU64::new(60).unwrap(), RoundingMode::HalfUp)
}
pub fn checked(v: D) -> Result<D, String> {
    if v.abs() >= *MAX_VALUE {
        return Err("Number exceeds 32 integer digits".into());
    }
    Ok(rounded(v))
}
pub fn div(a: &D, b: &D) -> Result<D, String> {
    if b.is_zero() {
        Err("Division by zero".into())
    } else {
        Ok(rounded(a / b))
    }
}
pub fn binary(a: &D, b: &D, op: char) -> Result<D, String> {
    let v = match op {
        '+' => a + b,
        '-' => a - b,
        '*' => rounded(a * b),
        '/' => div(a, b)?,
        '^' => power(a, b)?,
        _ => return Err("Unknown operator".into()),
    };
    checked(v)
}
pub fn power(a: &D, b: &D) -> Result<D, String> {
    if b.abs() > 10000 {
        return Err("Exponent is too large".into());
    }
    if b.is_integer() {
        let n = b.to_i64().ok_or("Exponent is too large")?;
        if a.is_zero() && n < 0 {
            return Err("Division by zero".into());
        }
        if let (Some(x), Some(y)) = (a.abs().to_f64(), b.to_f64())
            && x > 0.0
            && x.log10() * y >= 32.0
        {
            return Err("Number exceeds 32 integer digits".into());
        }
        return checked(a.powi(n));
    }
    if b == &*HALF {
        return a
            .sqrt()
            .map(rounded)
            .ok_or("Square root of a negative number".into());
    }
    if a <= &D::zero() {
        return Err("Fractional power needs a positive base".into());
    }
    exponential(&rounded(logarithm(a)? * b))
}
pub fn logarithm(x: &D) -> Result<D, String> {
    if x <= &D::zero() {
        return Err("Logarithm needs a positive value".into());
    }
    let mut v = x.clone();
    let mut n: i32 = 0;
    while v > *ONE_AND_HALF {
        v = rounded(v / 2);
        n += 1;
        if n > 500 {
            return Err("Value is too large".into());
        }
    }
    while v < *THREE_QUARTERS {
        v = rounded(v * 2);
        n -= 1;
        if n < -500 {
            return Err("Value is too small".into());
        }
    }
    let z = div(&(&v - 1), &(&v + 1))?;
    let z2 = rounded(&z * &z);
    let mut term = z.clone();
    let mut sum = z;
    for i in 1..250 {
        term = rounded(term * &z2);
        let add = rounded(&term / D::from(2 * i + 1));
        sum += &add;
        if add.abs() < *EPSILON {
            break;
        }
    }
    Ok(rounded(sum * 2 + D::from(n) * &*LN_TWO))
}
pub fn exponential(x: &D) -> Result<D, String> {
    if x > &D::from(74) {
        return Err("Number exceeds 32 integer digits".into());
    }
    if x < &D::from(-500) {
        return Err("Exponent is too small".into());
    }
    let negative = x < &D::zero();
    let mut v = x.abs();
    let mut squarings = 0;
    while v > *HALF {
        v = rounded(v / 2);
        squarings += 1;
    }
    let mut term = D::one();
    let mut sum = D::one();
    for i in 1..250 {
        term = rounded(term * &v / D::from(i));
        sum += &term;
        if term.abs() < *EPSILON {
            break;
        }
    }
    for _ in 0..squarings {
        sum = rounded(&sum * &sum);
    }
    if negative {
        sum = div(&D::one(), &sum)?;
    }
    checked(sum)
}
fn sin_cos(x: &D, cosine: bool) -> Result<D, String> {
    if x.abs() > 1_000_000 {
        return Err("Angle is too large".into());
    }
    let tau = &*TAU;
    let v = rounded(x - (&(x / tau).with_scale(0) * tau));
    let negative_square = -rounded(&v * &v);
    let mut term = if cosine { D::one() } else { v };
    let mut sum = term.clone();
    for i in 1..250 {
        let n = if cosine { 2 * i - 1 } else { 2 * i };
        term = rounded(term * &negative_square / D::from(n * (n + 1)));
        sum += &term;
        if term.abs() < *EPSILON {
            break;
        }
    }
    Ok(rounded(sum))
}
pub fn function(name: &str, x: &D) -> Result<D, String> {
    match name.to_ascii_lowercase().as_str() {
        "sqrt" => x
            .sqrt()
            .map(rounded)
            .ok_or("Square root of a negative number".into()),
        "abs" => Ok(x.abs()),
        "round" => Ok(x.with_scale_round(2, RoundingMode::HalfUp)),
        "ln" => logarithm(x),
        "log" => div(&logarithm(x)?, &LN_TEN),
        "exp" => exponential(x),
        "sin" => sin_cos(x, false),
        "cos" => sin_cos(x, true),
        "tan" => div(&sin_cos(x, false)?, &sin_cos(x, true)?),
        _ => Err(format!("Unknown function: {name}")),
    }
}
