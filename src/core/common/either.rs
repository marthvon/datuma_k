#[derive(Debug)]
pub enum Either<L, R> {
  Left(L),
  Right(R),
}

#[derive(Debug)]
pub enum Threather<L, M, R> {
  Left(L),
  Middle(M),
  Right(R),
}

#[derive(Debug)]
pub enum Feother<Aty, Bty, Cty, Dty> {
  A(Aty),
  B(Bty),
  C(Cty),
  D(Dty),
}
