//! spenso's `DataTensor`: every variant is routed to a `structure` of one
//! type, which the attribute cannot see. The partial block writes the
//! method returning `impl Iterator` through the enum's accessor, as the
//! sentence says, and leaves the provided method with a `Self` argument
//! to the trait's default, which calls the forwarded methods on both values
//! and so compares a dense and a sparse tensor by what they hold.

#[whichever::dispatchable]
pub trait TensorStructure {
    fn order(&self) -> usize;
    fn external_dims_iter(&self) -> impl Iterator<Item = usize>;
    fn same_external(&self, other: &Self) -> bool {
        self.external_dims_iter().eq(other.external_dims_iter())
    }
}

pub struct Ordered(pub Vec<usize>);

impl TensorStructure for Ordered {
    fn order(&self) -> usize {
        self.0.len()
    }
    fn external_dims_iter(&self) -> impl Iterator<Item = usize> {
        self.0.iter().copied()
    }
}

pub struct Dense<S> {
    pub structure: S,
    pub data: Vec<f64>,
}

pub struct Sparse<S> {
    pub structure: S,
    pub entries: Vec<(usize, f64)>,
}

#[whichever::dispatch(from = false)]
pub enum DataTensor<S> {
    #[dispatch(via = .structure)]
    Dense(Dense<S>),
    #[dispatch(via = .structure)]
    Sparse(Sparse<S>),
}

impl<S> DataTensor<S> {
    pub fn structure(&self) -> &S {
        match self {
            DataTensor::Dense(inner) => &inner.structure,
            DataTensor::Sparse(inner) => &inner.structure,
        }
    }
}

#[whichever::dispatch]
impl<S: TensorStructure> TensorStructure for DataTensor<S> {
    fn external_dims_iter(&self) -> impl Iterator<Item = usize> {
        self.structure().external_dims_iter()
    }

    #[dispatch(default)]
    fn same_external(&self, other: &Self) -> bool;
}

#[test]
fn a_dense_and_a_sparse_tensor_of_one_structure_compare_equal() {
    let dense = DataTensor::Dense(Dense {
        structure: Ordered(vec![2, 3]),
        data: vec![0.0; 6],
    });
    let sparse = DataTensor::Sparse(Sparse {
        structure: Ordered(vec![2, 3]),
        entries: vec![(4, 1.0)],
    });
    let other = DataTensor::Sparse(Sparse {
        structure: Ordered(vec![3, 2]),
        entries: vec![],
    });
    assert_eq!((dense.order(), sparse.order()), (2, 2));
    assert_eq!(dense.external_dims_iter().collect::<Vec<_>>(), [2, 3]);
    assert!(dense.same_external(&sparse));
    assert!(!sparse.same_external(&other));
}
