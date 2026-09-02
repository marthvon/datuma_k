use super::types::FactId;

pub fn union_sorted(a: &[FactId], b: &[FactId]) -> Vec<FactId> {
  let mut out = Vec::with_capacity(a.len() + b.len());
  let mut i = 0;
  let mut j = 0;
  while i < a.len() && j < b.len() {
    if a[i] < b[j] {
      out.push(a[i]);
      i += 1;
    } else if b[j] < a[i] {
      out.push(b[j]);
      j += 1;
    } else {
      out.push(a[i]);
      i += 1;
      j += 1;
    }
  }
  out.extend_from_slice(&a[i..]);
  out.extend_from_slice(&b[j..]);
  out
}

pub fn intersect_sorted(a: &[FactId], b: &[FactId]) -> Vec<FactId> {
  let mut out = Vec::with_capacity(a.len() + b.len());
  let mut i = 0;
  let mut j = 0;
  while i < a.len() && j < b.len() {
    if a[i] < b[j] {
      i += 1;
    } else if b[j] < a[i] {
      j += 1;
    } else {
      out.push(a[i]);
      i += 1;
      j += 1;
    }
  }
  out
}

pub fn difference_sorted(a: &[FactId], b: &[FactId]) -> Vec<FactId> {
  let mut out = Vec::with_capacity(a.len());
  let mut i = 0;
  let mut j = 0;
  while i < a.len() && j < b.len() {
    if a[i] < b[j] {
      out.push(a[i]);
      i += 1;
    } else if b[j] < a[i] {
      j += 1;
    } else {
      i += 1;
      j += 1;
    }
  }
  out.extend_from_slice(&a[i..]);
  out
}

#[cfg(test)]
mod tests {
  use super::{difference_sorted, intersect_sorted, union_sorted};

  #[test]
  fn union_disjoint() {
    assert_eq!(union_sorted(&[1, 3], &[2, 4]), [1, 2, 3, 4]);
  }

  #[test]
  fn union_identical() {
    assert_eq!(union_sorted(&[1, 2, 3], &[1, 2, 3]), [1, 2, 3]);
  }

  #[test]
  fn union_subset() {
    assert_eq!(union_sorted(&[1, 2, 4], &[2]), [1, 2, 4]);
  }

  #[test]
  fn union_empty() {
    assert_eq!(union_sorted(&[], &[1, 2]), vec![1, 2]);
    assert_eq!(union_sorted(&[1, 2], &[]), vec![1, 2]);
    assert_eq!(union_sorted(&[], &[]), Vec::<u32>::new());
  }

  #[test]
  fn union_interleaved() {
    assert_eq!(
      union_sorted(&[0, 2, 5, 9], &[1, 2, 3, 9, 10]),
      [0, 1, 2, 3, 5, 9, 10]
    );
  }

  #[test]
  fn intersect_disjoint() {
    assert_eq!(intersect_sorted(&[1, 3], &[2, 4]), Vec::<u32>::new());
  }

  #[test]
  fn intersect_identical() {
    assert_eq!(intersect_sorted(&[1, 2, 3], &[1, 2, 3]), [1, 2, 3]);
  }

  #[test]
  fn intersect_subset() {
    assert_eq!(intersect_sorted(&[1, 2, 4, 8], &[2, 8]), [2, 8]);
  }

  #[test]
  fn intersect_empty() {
    assert_eq!(intersect_sorted(&[], &[1, 2]), Vec::<u32>::new());
    assert_eq!(intersect_sorted(&[1, 2], &[]), Vec::<u32>::new());
  }

  #[test]
  fn intersect_interleaved() {
    assert_eq!(intersect_sorted(&[0, 2, 5, 9], &[1, 2, 3, 9, 10]), [2, 9]);
  }

  #[test]
  fn difference_disjoint() {
    assert_eq!(difference_sorted(&[1, 3], &[2, 4]), [1, 3]);
  }

  #[test]
  fn difference_identical() {
    assert_eq!(difference_sorted(&[1, 2, 3], &[1, 2, 3]), Vec::<u32>::new());
  }

  #[test]
  fn difference_subset() {
    assert_eq!(difference_sorted(&[1, 2, 4, 8], &[2, 8]), [1, 4]);
  }

  #[test]
  fn difference_empty() {
    assert_eq!(difference_sorted(&[], &[1, 2]), Vec::<u32>::new());
    assert_eq!(difference_sorted(&[1, 2], &[]), vec![1, 2]);
  }

  #[test]
  fn difference_interleaved() {
    assert_eq!(difference_sorted(&[0, 2, 5, 9], &[1, 2, 3, 9, 10]), [0, 5]);
  }
}
