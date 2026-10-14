use bigdecimal::{BigDecimal as D, RoundingMode};
use num_traits::{One, ToPrimitive, Zero};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    num::NonZeroU64,
    str::FromStr,
    sync::LazyLock,
};

pub type Number = D;
pub const MAX_SCALE: i64 = 512;
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
    validate(&v)?;
    Ok(rounded(v))
}
fn validate(v: &D) -> Result<(), String> {
    // Inspect the scale before comparison, rounding or plain-string expansion.
    // Significant precision and exponent bounds are separate constraints.
    let raw_scale = v.fractional_digit_count();
    if !(-MAX_SCALE - 60..=MAX_SCALE + 60).contains(&raw_scale) {
        return Err("Decimal exponent is outside the supported range (-512 to 512)".into());
    }
    // Most values already fit. Normalize only the rounding guard digits;
    // avoid allocating a new coefficient on every arithmetic operation.
    let scale = if (-MAX_SCALE..=MAX_SCALE).contains(&raw_scale) {
        raw_scale
    } else {
        v.normalized().fractional_digit_count()
    };
    if !(-MAX_SCALE..=MAX_SCALE).contains(&scale) {
        return Err("Decimal exponent is outside the supported range (-512 to 512)".into());
    }
    if v.to_ref().abs() >= MAX_VALUE.to_ref() {
        return Err("Number exceeds 32 integer digits".into());
    }
    Ok(())
}
pub fn div(a: &D, b: &D) -> Result<D, String> {
    validate(a)?;
    validate(b)?;
    if b.is_zero() {
        Err("Division by zero".into())
    } else {
        checked(a / b)
    }
}
pub fn binary(a: &D, b: &D, op: char) -> Result<D, String> {
    validate(a)?;
    validate(b)?;
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
    validate(a)?;
    validate(b)?;
    if b.abs() > 10000 {
        return Err("Exponent is too large".into());
    }
    if b.is_integer() {
        let n = b.to_i64().ok_or("Exponent is too large")?;
        if a.is_zero() && n < 0 {
            return Err("Division by zero".into());
        }
        // Bound intermediate work as well as the final answer. powi() can
        // construct enormous coefficients/scales before a final check runs.
        let mut base = if n < 0 { div(&D::one(), a)? } else { a.clone() };
        let mut remaining = n.unsigned_abs();
        let mut result = D::one();
        while remaining > 0 {
            if remaining & 1 == 1 {
                result = checked(result * &base)?;
            }
            remaining >>= 1;
            if remaining > 0 {
                base = checked(&base * &base)?;
            }
        }
        return Ok(result);
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
    validate(x)?;
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
    validate(x)?;
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
        // This bounded internal intermediate may exceed the public magnitude
        // limit even though its reciprocal is a valid small number.
        sum = rounded(D::one() / &sum);
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
#[derive(Default)]
struct FunctionCache {
    values: BTreeMap<(String, D), Result<D, String>>,
    order: VecDeque<(String, D)>,
}
thread_local! { static FUNCTIONS: RefCell<FunctionCache> = RefCell::default(); }
pub fn function(name: &str, x: &D) -> Result<D, String> {
    validate(x)?;
    let key = (name.to_ascii_lowercase(), x.normalized());
    if !matches!(
        key.0.as_str(),
        "sqrt" | "abs" | "round" | "ln" | "log" | "exp" | "sin" | "cos" | "tan"
    ) {
        return Err(format!("Unknown function: {name}"));
    }
    if let Some(result) = FUNCTIONS.with(|cache| cache.borrow().values.get(&key).cloned()) {
        return result;
    }
    let result = uncached_function(&key.0, x).and_then(checked);
    FUNCTIONS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.values.len() == 1024
            && let Some(old) = cache.order.pop_front()
        {
            cache.values.remove(&old);
        }
        cache.order.push_back(key.clone());
        cache.values.insert(key, result.clone());
    });
    result
}
fn uncached_function(name: &str, x: &D) -> Result<D, String> {
    match name {
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
