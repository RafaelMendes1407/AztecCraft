// Independent test oracle; deliberately shares no topology code with src/.
use std::collections::BTreeSet;

pub struct Mesh {
    pub points: Vec<[f32; 3]>,
    pub edges: Vec<[usize; 2]>,
    pub faces: Vec<Vec<i64>>,
}

impl Mesh {
    fn directed(&self, token: i64) -> Result<(usize, [usize; 2]), String> {
        // Maya reverses edge n with -n-1; complement also handles i64::MIN.
        let id = if token < 0 { !token } else { token };
        let id = usize::try_from(id).map_err(|_| format!("edge {token} out of range"))?;
        let [a, b] = *self
            .edges
            .get(id)
            .ok_or_else(|| format!("edge {token} out of range"))?;
        Ok((id, if token < 0 { [b, a] } else { [a, b] }))
    }

    pub fn loops(&self) -> Result<Vec<Vec<usize>>, String> {
        for (index, point) in self.points.iter().enumerate() {
            if point.iter().any(|v| !v.is_finite()) {
                return Err(format!("point {index} is not finite"));
            }
        }
        for (index, [a, b]) in self.edges.iter().enumerate() {
            if a == b || self.points.get(*a).is_none() || self.points.get(*b).is_none() {
                return Err(format!("edge {index} has invalid endpoints"));
            }
        }
        let mut used = BTreeSet::new();
        let mut loops = Vec::new();
        for (index, face) in self.faces.iter().enumerate() {
            if face.len() < 3 {
                return Err(format!("face {index} has fewer than 3 edges"));
            }
            let dirs = face
                .iter()
                .map(|t| self.directed(*t))
                .collect::<Result<Vec<_>, _>>()?;
            for (i, (_, [_, end])) in dirs.iter().enumerate() {
                if dirs[(i + 1) % dirs.len()].1[0] != *end {
                    return Err(format!("face {index} loop does not close at edge {i}"));
                }
                if !used.insert((dirs[i].0, face[i] < 0)) {
                    return Err(format!("directed edge {} used twice", face[i]));
                }
            }
            loops.push(dirs.iter().map(|(_, ends)| ends[0]).collect());
        }
        Ok(loops)
    }

    /// Closed requires both directions of each stored edge; open requires at least one.
    pub fn check(&self, closed: bool) -> Result<(), String> {
        self.loops()?;
        let used = self
            .faces
            .iter()
            .flatten()
            .map(|token| self.directed(*token).map(|(id, _)| (id, *token < 0)))
            .collect::<Result<BTreeSet<_>, _>>()?;
        for id in 0..self.edges.len() {
            let (forward, reverse) = (used.contains(&(id, false)), used.contains(&(id, true)));
            if (closed && !(forward && reverse)) || (!closed && !(forward || reverse)) {
                return Err(format!("edge {id} violates the boundary rule"));
            }
        }
        Ok(())
    }

    /// Fan triangulation gives positive volume for outward winding.
    pub fn signed_volume(&self) -> Result<f64, String> {
        let point = |i: usize| {
            self.points
                .get(i)
                .map(|p| p.map(f64::from))
                .ok_or_else(|| format!("point {i} out of range"))
        };
        let mut six = 0.0;
        for face in self.loops()? {
            for k in 1..face.len() - 1 {
                let (a, b, c) = (point(face[0])?, point(face[k])?, point(face[k + 1])?);
                six += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]);
            }
        }
        Ok(six / 6.0)
    }
}
