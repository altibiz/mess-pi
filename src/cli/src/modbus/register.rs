use std::fmt::Display;

use either::Either;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio_modbus::{Address, Quantity};

pub trait RegisterStorage {
  fn quantity(&self) -> Quantity;
}

pub trait Register {
  fn address(&self) -> Address;

  fn storage(&self) -> &dyn RegisterStorage;
}

pub trait UnparsedRegister<TParsed: Register>: Register {
  #[cfg(target_endian = "little")]
  fn parse<
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
  >(
    &self,
    data: &TIntoIterator,
  ) -> Option<TParsed>;

  #[cfg(target_endian = "big")]
  fn parse<TIntoIterator: IntoIterator<Item = u16>>(
    &self,
    data: &TIntoIterator,
  ) -> Option<TParsed>;
}

#[derive(Debug, Clone, Copy)]
pub struct StringRegisterKind {
  pub length: Quantity,
}

#[derive(Debug, Clone, Copy)]
pub struct NumericRegisterKind {
  pub multiplier: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum RegisterKind {
  U16(NumericRegisterKind),
  U32(NumericRegisterKind),
  U64(NumericRegisterKind),
  S16(NumericRegisterKind),
  S32(NumericRegisterKind),
  S64(NumericRegisterKind),
  F32(NumericRegisterKind),
  F64(NumericRegisterKind),
  String(StringRegisterKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RegisterValue {
  U16(u16),
  U32(u32),
  U64(u64),
  S16(i16),
  S32(i32),
  S64(i64),
  F32(f32),
  F64(f64),
  String(String),
}

#[derive(Debug, Clone)]
pub struct MeasurementRegister<T: RegisterStorage> {
  pub address: Address,
  pub storage: T,
  pub name: String,
}

#[derive(Debug, Clone)]
pub struct DetectRegister<T: RegisterStorage> {
  pub address: Address,
  pub storage: T,
  pub r#match: Either<String, Regex>,
}

#[derive(Debug, Clone)]
pub struct IdRegister<T: RegisterStorage> {
  pub address: Address,
  pub storage: T,
}

impl RegisterStorage for RegisterKind {
  fn quantity(&self) -> Quantity {
    match self {
      RegisterKind::U16(_) => 1,
      RegisterKind::U32(_) => 2,
      RegisterKind::U64(_) => 4,
      RegisterKind::S16(_) => 1,
      RegisterKind::S32(_) => 2,
      RegisterKind::S64(_) => 4,
      RegisterKind::F32(_) => 2,
      RegisterKind::F64(_) => 4,
      RegisterKind::String(StringRegisterKind { length }) => *length,
    }
  }
}

impl RegisterStorage for RegisterValue {
  fn quantity(&self) -> Quantity {
    match self {
      RegisterValue::U16(_) => 1,
      RegisterValue::U32(_) => 2,
      RegisterValue::U64(_) => 4,
      RegisterValue::S16(_) => 1,
      RegisterValue::S32(_) => 2,
      RegisterValue::S64(_) => 4,
      RegisterValue::F32(_) => 2,
      RegisterValue::F64(_) => 4,
      RegisterValue::String(value) => value.len() as Quantity,
    }
  }
}

impl Display for RegisterValue {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> Result<(), std::fmt::Error> {
    match self {
      RegisterValue::U16(value) => value.fmt(f),
      RegisterValue::U32(value) => value.fmt(f),
      RegisterValue::U64(value) => value.fmt(f),
      RegisterValue::S16(value) => value.fmt(f),
      RegisterValue::S32(value) => value.fmt(f),
      RegisterValue::S64(value) => value.fmt(f),
      RegisterValue::F32(value) => value.fmt(f),
      RegisterValue::F64(value) => value.fmt(f),
      RegisterValue::String(value) => value.fmt(f),
    }
  }
}

impl DetectRegister<RegisterValue> {
  pub fn matches(&self) -> bool {
    let storage = self.storage.to_string();
    match self.r#match {
      Either::Left(string) => string == storage,
      Either::Right(regex) => regex.is_match(storage.as_str()),
    }
  }
}

impl IdRegister<RegisterValue> {
  pub fn id(&self) -> String {
    self.storage.to_string()
  }
}

pub fn serialize_registers<T>(registers: T) -> serde_json::Value
where
  T: IntoIterator<Item = MeasurementRegister<RegisterValue>>,
{
  serde_json::Value::Object(
    registers
      .into_iter()
      .map(
        |MeasurementRegister::<RegisterValue> { name, storage, .. }| {
          (name.clone(), serde_json::json!(storage))
        },
      )
      .collect::<serde_json::Map<String, serde_json::Value>>(),
  )
}

macro_rules! impl_register {
  ($type: ident) => {
    impl<T: RegisterStorage> Register for $type<T> {
      fn address(&self) -> Address {
        self.address
      }

      fn storage(&self) -> &dyn RegisterStorage {
        &self.storage
      }
    }

    impl Display for $type<RegisterValue> {
      fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
      ) -> Result<(), std::fmt::Error> {
        self.storage.fmt(f)
      }
    }
  };
}

impl_register!(MeasurementRegister);
impl_register!(DetectRegister);
impl_register!(IdRegister);

macro_rules! parse_integer_register_kind {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident) => {{
    let bytes = parse_numeric_bytes($data);
    let slice = bytes.as_slice().try_into().ok()?;
    let value = <$type>::from_ne_bytes(slice);
    RegisterValue::$variant(match $multiplier {
      Some($multiplier) => ((value as f64) * $multiplier).round() as $type,
      None => value,
    })
  }};
}

macro_rules! parse_floating_register_kind {
  ($variant: ident, $type: ty, $data: ident, $multiplier: ident) => {{
    let bytes = parse_numeric_bytes($data);
    let slice = bytes.as_slice().try_into().ok()?;
    let value = <$type>::from_ne_bytes(slice);
    RegisterValue::$variant(match $multiplier {
      Some($multiplier) => ((value as f64) * $multiplier) as $type,
      None => value,
    })
  }};
}

macro_rules! impl_parse_register {
  ($type: ident, $result: expr) => {
    #[cfg(target_endian = "little")]
    impl UnparsedRegister<$type<RegisterValue>> for $type<RegisterKind> {
      fn parse<
        TIterator: DoubleEndedIterator<Item = u16>,
        TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>,
      >(
        &self,
        data: &TIntoIterator,
      ) -> Option<$type<RegisterValue>> {
        let value = match self.storage {
          RegisterKind::U16(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(U16, u16, data, multiplier)
          }
          RegisterKind::U32(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(U32, u32, data, multiplier)
          }
          RegisterKind::U64(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(U64, u64, data, multiplier)
          }
          RegisterKind::S16(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(S16, i16, data, multiplier)
          }
          RegisterKind::S32(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(S32, i32, data, multiplier)
          }
          RegisterKind::S64(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(S64, i64, data, multiplier)
          }
          RegisterKind::F32(NumericRegisterKind { multiplier }) => {
            parse_floating_register_kind!(F32, f32, data, multiplier)
          }
          RegisterKind::F64(NumericRegisterKind { multiplier }) => {
            parse_floating_register_kind!(F64, f64, data, multiplier)
          }
          RegisterKind::String(_) => {
            let bytes = parse_string_bytes(data);
            RegisterValue::String(String::from_utf8(bytes).ok()?)
          }
        };

        Some($result(self, value))
      }
    }

    #[cfg(target_endian = "big")]
    impl UnparsedRegister<$type<RegisterValue>> for $type<RegisterKind> {
      fn parse<TIntoIterator: IntoIterator<Item = u16>>(
        &self,
        data: &TIntoIterator,
      ) -> Option<$type<RegisterValue>> {
        let value = match self.storage {
          RegisterKind::U16(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(U16, u16, data, multiplier)
          }
          RegisterKind::U32(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(U32, u32, data, multiplier)
          }
          RegisterKind::U64(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(U64, u64, data, multiplier)
          }
          RegisterKind::S16(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(S16, i16, data, multiplier)
          }
          RegisterKind::S32(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(S32, i32, data, multiplier)
          }
          RegisterKind::S64(NumericRegisterKind { multiplier }) => {
            parse_integer_register_kind!(S64, i64, data, multiplier)
          }
          RegisterKind::F32(NumericRegisterKind { multiplier }) => {
            parse_floating_register_kind!(F32, f32, data, multiplier)
          }
          RegisterKind::F64(NumericRegisterKind { multiplier }) => {
            parse_floating_register_kind!(F64, f64, data, multiplier)
          }
          RegisterKind::String(_) => {
            let bytes = parse_string_bytes(data);
            RegisterValue::String(String::from_utf8(bytes).ok()?)
          }
        };

        Some($result(self, value))
      }
    }
  };
}

impl_parse_register!(
  MeasurementRegister,
  |&MeasurementRegister::<RegisterKind> { address, name, .. }, storage| {
    MeasurementRegister::<RegisterValue> {
      address,
      storage,
      name,
    }
  }
);
impl_parse_register!(
  DetectRegister,
  |&DetectRegister::<RegisterKind> {
     address, r#match, ..
   },
   storage| {
    DetectRegister::<RegisterValue> {
      address,
      storage,
      r#match,
    }
  }
);
impl_parse_register!(IdRegister, |&IdRegister::<RegisterKind> {
                                    address,
                                    ..
                                  },
                                  storage| {
  IdRegister::<RegisterValue> { address, storage }
});

#[cfg(target_endian = "little")]
fn parse_numeric_bytes<
  I: DoubleEndedIterator<Item = u16>,
  T: IntoIterator<Item = u16, IntoIter = I>,
>(
  data: &T,
) -> Vec<u8> {
  data
    .into_iter()
    .rev()
    .map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .flatten()
    .collect()
}

#[cfg(target_endian = "big")]
fn parse_numeric_bytes<T: IntoIterator<Item = u16>>(data: &T) -> Vec<u8> {
  data
    .into_iter()
    .map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .flatten()
    .collect()
}

#[cfg(target_endian = "little")]
fn parse_string_bytes<T: IntoIterator<Item = u16>>(data: &T) -> Vec<u8> {
  data
    .into_iter()
    .map(|value| [(value >> 8) as u8, (value & 0xFF) as u8])
    .flatten()
    .collect()
}

#[cfg(target_endian = "big")]
fn parse_string_bytes<T: IntoIterator<Item = u16>>(data: &T) -> Vec<u8> {
  data
    .into_iter()
    .map(|value| [(value & 0xFF) as u8, (value >> 8) as u8])
    .flatten()
    .collect()
}
