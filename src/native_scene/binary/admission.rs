use super::{BinaryError, resource};
use std::mem::size_of;

const MAX_BYTES: usize = 128 * 1024 * 1024;
const MAX_WORK: usize = 50_000_000;

#[derive(Default)]
pub(super) struct Counts {
    pub transforms: usize,
    pub shapes: usize,
    pub dag: usize,
    pub atoms: usize,
    pub leaves: usize,
    pub points: usize,
    pub edges: usize,
    pub faces: usize,
    pub corners: usize,
    pub offsets: usize,
    pub chunks: usize,
    pub records: usize,
}

fn sum(values: impl IntoIterator<Item = usize>) -> Result<usize, BinaryError> {
    values.into_iter().try_fold(0usize, |total, value| {
        total.checked_add(value).ok_or_else(resource)
    })
}

fn product(a: usize, b: usize) -> Result<usize, BinaryError> {
    a.checked_mul(b).ok_or_else(resource)
}

impl Counts {
    pub fn admit_work(&self) -> Result<(), BinaryError> {
        let v = sum([self.transforms, self.shapes])?;
        let pairs = if self.leaves == 0 {
            0
        } else {
            product(self.leaves, self.leaves - 1)? / 2
        };
        let linear = sum([
            self.chunks,
            self.records,
            self.transforms,
            self.shapes,
            self.dag,
            self.atoms,
            self.leaves,
            self.points,
            self.edges,
            self.faces,
            self.corners,
            self.offsets,
        ])?;
        let work = sum([
            product(product(2, v)?, self.dag)?,
            product(self.transforms, self.leaves)?,
            pairs,
            product(32, linear)?,
        ])?;
        if work > MAX_WORK {
            return Err(resource());
        }
        Ok(())
    }

    pub fn validation_bytes(&self) -> Result<usize, BinaryError> {
        let v = sum([self.transforms, self.shapes])?;
        sum([
            4096,
            product(
                256,
                sum([product(3, self.transforms)?, product(3, v)?, self.dag])?,
            )?,
        ])
    }
}

pub(super) struct Budget {
    used: usize,
}

impl Budget {
    pub fn snapshot(&self) -> Self {
        Self { used: self.used }
    }
    pub fn new(bytes: usize) -> Result<Self, BinaryError> {
        let mut budget = Self { used: 0 };
        budget.charge(bytes)?;
        Ok(budget)
    }

    pub fn charge(&mut self, bytes: usize) -> Result<(), BinaryError> {
        self.used = self
            .used
            .checked_add(bytes)
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(resource)?;
        Ok(())
    }

    pub fn preflight<T>(&mut self, count: usize) -> Result<(), BinaryError> {
        self.charge(product(count, size_of::<T>())?)
    }

    pub fn vector<T>(&mut self, count: usize) -> Result<Vec<T>, BinaryError> {
        let bytes = product(count, size_of::<T>())?;
        self.used
            .checked_add(bytes)
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(resource)?;
        let mut vector = Vec::new();
        vector.try_reserve_exact(count).map_err(|_| resource())?;
        self.charge(product(vector.capacity(), size_of::<T>())?)?;
        Ok(vector)
    }

    pub fn string(&mut self, text: &str) -> Result<String, BinaryError> {
        self.used
            .checked_add(text.len())
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(resource)?;
        let mut value = String::new();
        value
            .try_reserve_exact(text.len())
            .map_err(|_| resource())?;
        self.charge(value.capacity())?;
        value.push_str(text);
        Ok(value)
    }
}
