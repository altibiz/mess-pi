use std::fmt::Debug;

use tokio_modbus::{Address, Quantity};

pub trait Span: Debug + Send {
  fn address(&self) -> Address;

  fn quantity(&self) -> Quantity;
}

pub trait SpanParser<TParsed: Span>: Span + Clone + Debug + Send {
  fn parse<TIterator, TIntoIterator>(
    &self,
    data: TIntoIterator,
  ) -> anyhow::Result<TParsed>
  where
    TIterator: DoubleEndedIterator<Item = u16>,
    TIntoIterator: IntoIterator<Item = u16, IntoIter = TIterator>;
}
