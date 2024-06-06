#![allow(dead_code)]

use crate::{MAX_WIRE_WIDTH, MIN_WIRE_WIDTH};
use bytemuck::{Pod, Zeroable};
use std::fmt::{self, Write};

/// The logic state of a single bit
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LogicBitState {
    /// The high impedance state
    #[default]
    HighZ = 0b00,
    /// An undefined logic level
    Undefined = 0b01,
    /// The low logic level
    Logic0 = 0b10,
    /// The high logic level
    Logic1 = 0b11,
}

impl LogicBitState {
    #[inline]
    const fn from_bits(state_bit: bool, valid_bit: bool) -> Self {
        match (state_bit, valid_bit) {
            (false, false) => Self::HighZ,
            (true, false) => Self::Undefined,
            (false, true) => Self::Logic0,
            (true, true) => Self::Logic1,
        }
    }

    #[inline]
    const fn to_bits(self) -> (bool, bool) {
        let state = (self as u8) & 0x1;
        let valid = (self as u8) >> 1;
        (state > 0, valid > 0)
    }

    /// Creates a logic bit state representing a boolean value
    #[inline]
    pub const fn from_bool(value: bool) -> Self {
        match value {
            false => Self::Logic0,
            true => Self::Logic1,
        }
    }

    /// The boolean value this logic bit state represents, if any
    #[inline]
    pub const fn to_bool(self) -> Option<bool> {
        match self {
            Self::HighZ | Self::Undefined => None,
            Self::Logic0 => Some(false),
            Self::Logic1 => Some(true),
        }
    }

    /// - `b'Z'` | `b'z'` => `HighZ`
    /// - `b'X'` | `b'x'` => `Undefined`
    /// - `b'0'` => `Logic0`
    /// - `b'1'` => `Logic1`
    #[inline]
    pub const fn parse_byte(c: u8) -> Option<Self> {
        match c {
            b'Z' | b'z' => Some(Self::HighZ),
            b'X' | b'x' => Some(Self::Undefined),
            b'0' => Some(Self::Logic0),
            b'1' => Some(Self::Logic1),
            _ => None,
        }
    }

    /// - `'Z'` | `'z'` => `HighZ`
    /// - `'X'` | `'x'` => `Undefined`
    /// - `'0'` => `Logic0`
    /// - `'1'` => `Logic1`
    #[inline]
    pub const fn parse(c: char) -> Option<Self> {
        if c.is_ascii() {
            Self::parse_byte(c as u8)
        } else {
            None
        }
    }

    /// - `HighZ` => `'Z'`
    /// - `Undefined` => `'X'`
    /// - `Logic0` => `'0'`
    /// - `Logic1` => `'1'`
    #[inline]
    pub const fn to_char(self) -> char {
        match self {
            LogicBitState::HighZ => 'Z',
            LogicBitState::Undefined => 'X',
            LogicBitState::Logic0 => '0',
            LogicBitState::Logic1 => '1',
        }
    }
}

impl From<bool> for LogicBitState {
    #[inline]
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

impl TryFrom<LogicBitState> for bool {
    type Error = ();

    #[inline]
    fn try_from(value: LogicBitState) -> Result<Self, Self::Error> {
        value.to_bool().ok_or(())
    }
}

impl fmt::Display for LogicBitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char(self.to_char())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for LogicBitState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_char(self.to_char())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for LogicBitState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::*;

        struct LogicBitStateVisitor;

        impl Visitor<'_> for LogicBitStateVisitor {
            type Value = LogicBitState;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("one of the chars ['Z', 'z', 'X', 'x', '0', '1']")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v.len() == 1 {
                    LogicBitState::parse_byte(v.as_bytes()[0])
                } else {
                    None
                }
                .ok_or_else(|| E::invalid_value(Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(LogicBitStateVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Zeroable, Pod)]
#[repr(C)]
pub struct LogicStateAtom {
    //  state | valid | meaning
    // -------|-------|---------
    //    0   |   0   | High-Z
    //    1   |   0   | Undefined
    //    0   |   1   | Logic 0
    //    1   |   1   | Logic 1
    state: u32,
    valid: u32,
}

impl LogicStateAtom {
    pub const BITS: u32 = u32::BITS;

    pub const HIGH_Z: Self = Self {
        state: 0x00000000,
        valid: 0x00000000,
    };

    pub const UNDEFINED: Self = Self {
        state: 0xFFFFFFFF,
        valid: 0x00000000,
    };

    pub const LOGIC_0: Self = Self {
        state: 0x00000000,
        valid: 0xFFFFFFFF,
    };

    pub const LOGIC_1: Self = Self {
        state: 0xFFFFFFFF,
        valid: 0xFFFFFFFF,
    };

    #[inline]
    pub const fn from_int(value: u32) -> Self {
        Self {
            state: value,
            valid: 0xFFFFFFFF,
        }
    }

    #[inline]
    pub const fn from_bool(value: bool) -> Self {
        Self::from_int(value as u32)
    }

    fn from_bits(bits: &[LogicBitState]) -> Self {
        debug_assert!(!bits.is_empty());
        debug_assert!(bits.len() <= (Self::BITS as usize));

        let mut state = 0;
        let mut valid = 0;

        for bit in bits {
            state <<= 1;
            valid <<= 1;

            let (bit_state, bit_valid) = bit.to_bits();

            state |= bit_state as u32;
            valid |= bit_valid as u32;
        }

        Self { state, valid }
    }

    fn parse(s: &[u8]) -> Result<Self, ParseError> {
        debug_assert!(!s.is_empty());
        debug_assert!(s.len() <= (Self::BITS as usize));

        let mut state = 0;
        let mut valid = 0;

        for &c in s {
            state <<= 1;
            valid <<= 1;

            let bit = LogicBitState::parse_byte(c).ok_or(ParseError::IllegalCharacter(c))?;
            let (bit_state, bit_valid) = bit.to_bits();

            state |= bit_state as u32;
            valid |= bit_valid as u32;
        }

        Ok(Self {
            state: state,
            valid: valid,
        })
    }

    #[inline]
    const fn get_bit_state(&self, bit_index: u32) -> LogicBitState {
        let state_bit = ((self.state >> bit_index) & 0x1) > 0;
        let valid_bit = ((self.valid >> bit_index) & 0x1) > 0;
        LogicBitState::from_bits(state_bit, valid_bit)
    }
}

impl fmt::Display for LogicStateAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in (0..Self::BITS).rev() {
            let bit = self.get_bit_state(i);
            write!(f, "{bit}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FromBigIntError {
    /// The number of words was not between 1 and 8 inclusive
    InvalidWordCount,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FromBitsError {
    /// The number of bits was not between 1 and 256 inclusive
    InvalidWidth,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// The string contained a character other than `x`, `X`, `z`, `Z`, `0` or `1`
    IllegalCharacter(u8),
    /// The number of bits was not between 1 and 256 inclusive
    InvalidWidth,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToIntError {
    /// The width was not between 1 and 256 inclusive
    InvalidWidth,
    /// The first `width` bits of the logic state are not representable by an integer
    Unrepresentable,
}

const MAX_ATOM_COUNT: usize = (MAX_WIRE_WIDTH / LogicStateAtom::BITS) as usize;

/// A `MAX_WIRE_WIDTH` bit wide logic state
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct LogicState(pub(crate) [LogicStateAtom; MAX_ATOM_COUNT]);

impl LogicState {
    /// A logic state representing high impedance on all bits
    pub const HIGH_Z: Self = Self([LogicStateAtom::HIGH_Z; MAX_ATOM_COUNT]);

    /// A logic state representing an undefined logic level on all bits
    pub const UNDEFINED: Self = Self([LogicStateAtom::UNDEFINED; MAX_ATOM_COUNT]);

    /// A logic state representing a low logic level on all bits
    pub const LOGIC_0: Self = Self([LogicStateAtom::LOGIC_0; MAX_ATOM_COUNT]);

    /// A logic state representing a high logic level on all bits
    pub const LOGIC_1: Self = Self([LogicStateAtom::LOGIC_1; MAX_ATOM_COUNT]);

    /// Creates a new logic state representing the given integer value
    ///
    /// Bits past the first 32 are assigned the value 0
    #[inline]
    pub const fn from_int(value: u32) -> Self {
        Self([
            LogicStateAtom::from_int(value),
            LogicStateAtom::LOGIC_0,
            LogicStateAtom::LOGIC_0,
            LogicStateAtom::LOGIC_0,
            LogicStateAtom::LOGIC_0,
            LogicStateAtom::LOGIC_0,
            LogicStateAtom::LOGIC_0,
            LogicStateAtom::LOGIC_0,
        ])
    }

    /// Creates a new logic state representing the given boolean value
    ///
    /// Bits past the first one are assigned the value 0
    #[inline]
    pub const fn from_bool(value: bool) -> Self {
        Self::from_int(value as u32)
    }

    /// Creates a new logic state representing the given integer value
    ///
    /// Integer words are given in little endian order, bits past the end are assigned the value 0
    pub fn from_big_int(value: &[u32]) -> Result<Self, FromBigIntError> {
        if (1..=MAX_ATOM_COUNT).contains(&value.len()) {
            let mut this = Self::LOGIC_0;
            for (dst, &src) in this.0.iter_mut().zip(value) {
                dst.state = src;
            }
            Ok(this)
        } else {
            Err(FromBigIntError::InvalidWordCount)
        }
    }

    /// Creates a new logic state from the given bits (most significant bit first)
    ///
    /// Bits past the specified ones are implicitely assigned the value Z
    ///
    /// ### Example:
    /// ```
    /// use gsim2::{LogicState, LogicBitState};
    ///
    /// let state = LogicState::from_bits(&[
    ///     LogicBitState::Logic1,
    ///     LogicBitState::Logic0,
    ///     LogicBitState::Undefined,
    ///     LogicBitState::HighZ,
    /// ]).unwrap();
    /// assert_eq!(state.to_string(5), "Z10XZ");
    /// ```
    pub fn from_bits(bits: &[LogicBitState]) -> Result<Self, FromBitsError> {
        if !((MIN_WIRE_WIDTH as usize)..=(MAX_WIRE_WIDTH as usize)).contains(&bits.len()) {
            return Err(FromBitsError::InvalidWidth);
        }

        let width = bits.len() as u32;
        let head_width = (width % LogicStateAtom::BITS) as usize;
        let list_len = width.div_ceil(LogicStateAtom::BITS) as usize;

        let mut atoms = [LogicStateAtom::HIGH_Z; MAX_ATOM_COUNT];
        let mut i = list_len;

        if head_width > 0 {
            let head_bits = &bits[..head_width];
            let head = LogicStateAtom::from_bits(head_bits);

            i -= 1;
            atoms[i] = head;
        }

        let tail_bits = &bits[head_width..];
        let mut chunks = tail_bits.chunks_exact(LogicStateAtom::BITS as usize);
        for item_bits in chunks.by_ref() {
            let item = LogicStateAtom::from_bits(item_bits);

            i -= 1;
            atoms[i] = item;
        }
        debug_assert_eq!(chunks.remainder().len(), 0);

        Ok(Self(atoms))
    }

    /// Constructs a logic state from a string of bits (most significant bit first)
    ///
    /// Bits past the specified ones are implicitely assigned the value Z
    ///
    /// ### Example:
    /// ```
    /// use gsim2::LogicState;
    ///
    /// let state = LogicState::parse("10XZ").unwrap();
    /// assert_eq!(state.to_string(5), "Z10XZ");
    /// ```
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let s = s.as_bytes();

        if !((MIN_WIRE_WIDTH as usize)..=(MAX_WIRE_WIDTH as usize)).contains(&s.len()) {
            return Err(ParseError::InvalidWidth);
        }

        let width = s.len() as u32;
        let head_width = (width % LogicStateAtom::BITS) as usize;
        let list_len = width.div_ceil(LogicStateAtom::BITS) as usize;

        let mut atoms = [LogicStateAtom::HIGH_Z; MAX_ATOM_COUNT];
        let mut i = list_len;

        if head_width > 0 {
            let head_str = &s[..head_width];
            let head = LogicStateAtom::parse(head_str)?;

            i -= 1;
            atoms[i] = head;
        }

        let tail_str = &s[head_width..];
        let mut chunks = tail_str.chunks_exact(LogicStateAtom::BITS as usize);
        for item_bits in chunks.by_ref() {
            let item = LogicStateAtom::parse(item_bits)?;

            i -= 1;
            atoms[i] = item;
        }
        debug_assert_eq!(chunks.remainder().len(), 0);

        Ok(Self(atoms))
    }

    /// Converts the first `width` bits of the logic state into an integer
    ///
    /// ### Example:
    /// ```
    /// use gsim2::{LogicState, ToIntError};
    ///
    /// assert_eq!(LogicState::HIGH_Z.to_int(32), Err(ToIntError::Unrepresentable));
    /// assert_eq!(LogicState::UNDEFINED.to_int(32), Err(ToIntError::Unrepresentable));
    /// assert_eq!(LogicState::LOGIC_0.to_int(32), Ok(u32::MIN));
    /// assert_eq!(LogicState::LOGIC_1.to_int(32), Ok(u32::MAX));
    /// ```
    pub const fn to_int(&self, width: u32) -> Result<u32, ToIntError> {
        if (width < MIN_WIRE_WIDTH) || (width > u32::BITS) {
            return Err(ToIntError::InvalidWidth);
        }

        let mask = ((1u64 << width) - 1) as u32;
        if (self.0[0].valid & mask) == mask {
            Ok(self.0[0].state & mask)
        } else {
            Err(ToIntError::Unrepresentable)
        }
    }

    /// Converts the first bit of the logic state into a boolean
    ///
    /// ### Example:
    /// ```
    /// use gsim2::LogicState;
    ///
    /// assert_eq!(LogicState::HIGH_Z.to_bool(), None);
    /// assert_eq!(LogicState::UNDEFINED.to_bool(), None);
    /// assert_eq!(LogicState::LOGIC_0.to_bool(), Some(false));
    /// assert_eq!(LogicState::LOGIC_1.to_bool(), Some(true));
    /// ```
    pub const fn to_bool(&self) -> Option<bool> {
        self.0[0].get_bit_state(0).to_bool()
    }

    /// Converts the first `width` bits of the logic state into an integer
    ///
    /// Integer words are given in little endian order
    pub fn to_big_int<T: FromIterator<u32>>(&self, width: u32) -> Result<T, ToIntError> {
        if (width < MIN_WIRE_WIDTH) || (width > MAX_WIRE_WIDTH) {
            return Err(ToIntError::InvalidWidth);
        }

        let word_count = width.div_ceil(LogicStateAtom::BITS) as usize;

        let last_index = (width / LogicStateAtom::BITS) as usize;
        let last_width = width % LogicStateAtom::BITS;
        let last_mask = ((1u64 << last_width) - 1) as u32;

        self.0[..word_count]
            .iter()
            .enumerate()
            .map(|(i, atom)| {
                let mask = if i == last_index { last_mask } else { u32::MAX };

                if (atom.valid & mask) == mask {
                    Ok(atom.state & mask)
                } else {
                    Err(ToIntError::Unrepresentable)
                }
            })
            .collect()
    }

    /// Gets the logic state of a single bit
    pub const fn get_bit_state(&self, bit_index: u8) -> LogicBitState {
        let atom_index = (bit_index as usize) / (LogicStateAtom::BITS as usize);
        let bit_index = (bit_index as u32) % LogicStateAtom::BITS;
        self.0[atom_index].get_bit_state(bit_index)
    }

    /// Creates a string representing the first `width` bits of this state
    pub fn to_string(&self, width: u32) -> String {
        assert!(
            (width >= MIN_WIRE_WIDTH) && (width <= MAX_WIRE_WIDTH),
            "invalid bit width",
        );

        let mut s = String::with_capacity(width as usize);
        for i in (0..width).rev() {
            let bit = self.get_bit_state(i as u8);
            s.push(bit.to_char());
        }
        s
    }

    /// Tests the first `width` bits of this state and another for equality
    pub fn eq(&self, other: &Self, width: u32) -> bool {
        assert!(
            (width >= MIN_WIRE_WIDTH) && (width <= MAX_WIRE_WIDTH),
            "invalid bit width",
        );

        let atom_count = width.div_ceil(LogicStateAtom::BITS) as usize;

        let last_index = (width / LogicStateAtom::BITS) as usize;
        let last_width = width % LogicStateAtom::BITS;
        let last_mask = ((1u64 << last_width) - 1) as u32;

        for (i, (a, b)) in self.0.into_iter().zip(other.0).enumerate().take(atom_count) {
            let mask = if i == last_index { last_mask } else { u32::MAX };

            if ((a.state & mask) != (b.state & mask)) || ((a.valid & mask) != (b.valid & mask)) {
                return false;
            }
        }

        true
    }
}

impl Default for LogicState {
    #[inline]
    fn default() -> Self {
        Self::HIGH_Z
    }
}

impl From<bool> for LogicState {
    #[inline]
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

impl TryFrom<LogicState> for bool {
    type Error = ();

    #[inline]
    fn try_from(value: LogicState) -> Result<Self, Self::Error> {
        value.to_bool().ok_or(())
    }
}

impl From<u32> for LogicState {
    #[inline]
    fn from(value: u32) -> Self {
        Self::from_int(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for LogicState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string(MAX_WIRE_WIDTH))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for LogicState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::*;

        struct LogicStateVisitor;

        impl Visitor<'_> for LogicStateVisitor {
            type Value = LogicState;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a string consisting of only the chars ['Z', 'z', 'X', 'x', '0', '1'] and length {MIN_WIRE_WIDTH} to {MAX_WIRE_WIDTH}")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                LogicState::parse(v).map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(LogicStateVisitor)
    }
}

/// Constructs a logic state from a list of bits (most significant bit first)
///
/// ### Example:
/// ```
/// let state = gsim2::bits![1, 0, X, Z];
/// assert_eq!(state.to_string(4), "10XZ");
/// ```
#[macro_export]
macro_rules! bits {
    (@BIT Z) => { $crate::LogicBitState::HighZ };
    (@BIT z) => { $crate::LogicBitState::HighZ };
    (@BIT X) => { $crate::LogicBitState::Undefined };
    (@BIT x) => { $crate::LogicBitState::Undefined };
    (@BIT 0) => { $crate::LogicBitState::Logic0 };
    (@BIT 1) => { $crate::LogicBitState::Logic1 };
    ($($bit:tt),+) => {{
        const BITS: &'static [$crate::LogicBitState] = &[$($crate::bits!(@BIT $bit)),+];
        const _ASSERT_MAX: usize = (u8::MAX as usize) - BITS.len();
        const _ASSERT_MIN: usize = BITS.len() - 1;
        $crate::LogicState::from_bits(BITS).unwrap()
    }}
}
