/// A single face outcome when a die is rolled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    /// Special character – beats all numbers.
    Minotaur,
    Griffin,
    Mermaid,
    /// Numeric value (1–7).
    Number(u8),
    /// Flag = 0; can never win a trick.
    Flag,
}

impl Face {
    /// Returns true if this face is a special character (Minotaur / Griffin / Mermaid).
    pub fn is_special(self) -> bool {
        matches!(self, Face::Minotaur | Face::Griffin | Face::Mermaid)
    }

    /// Returns true if this face counts as a number (not a flag, not special).
    pub fn is_number(self) -> bool {
        matches!(self, Face::Number(_))
    }

    /// Numeric value: special characters return 0 for comparison purposes within their tier.
    pub fn numeric_value(self) -> u8 {
        match self {
            Face::Number(n) => n,
            _ => 0,
        }
    }
}

// ── Die type ─────────────────────────────────────────────────────────────────

/// The seven distinct die types in the Mino Dice bag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DieType {
    Minotaur,
    Griffin,
    Mermaid,
    Red,
    Yellow,
    Purple,
    Gray,
}

impl DieType {
    /// Number of this die type present in the bag.
    pub fn bag_count(self) -> u8 {
        match self {
            DieType::Minotaur => 1,
            DieType::Griffin => 3,
            DieType::Mermaid => 2,
            DieType::Red => 7,
            DieType::Yellow => 7,
            DieType::Purple => 8,
            DieType::Gray => 8,
        }
    }

    /// All 6 faces of this die type (in order, duplicates allowed).
    pub fn faces(self) -> [Face; 6] {
        match self {
            DieType::Minotaur => [
                Face::Minotaur,
                Face::Minotaur,
                Face::Minotaur,
                Face::Minotaur,
                Face::Flag,
                Face::Flag,
            ],
            DieType::Griffin => [
                Face::Griffin,
                Face::Griffin,
                Face::Griffin,
                Face::Griffin,
                Face::Flag,
                Face::Flag,
            ],
            DieType::Mermaid => [
                Face::Mermaid,
                Face::Mermaid,
                Face::Mermaid,
                Face::Mermaid,
                Face::Flag,
                Face::Flag,
            ],
            // Red: 7×2, 6×2, 5×2
            DieType::Red => [
                Face::Number(7),
                Face::Number(7),
                Face::Number(6),
                Face::Number(6),
                Face::Number(5),
                Face::Number(5),
            ],
            // Yellow: 5×2, 4×2, 3×2
            DieType::Yellow => [
                Face::Number(5),
                Face::Number(5),
                Face::Number(4),
                Face::Number(4),
                Face::Number(3),
                Face::Number(3),
            ],
            // Purple: 3×2, 2×2, 1×2
            DieType::Purple => [
                Face::Number(3),
                Face::Number(3),
                Face::Number(2),
                Face::Number(2),
                Face::Number(1),
                Face::Number(1),
            ],
            // Gray: Flag×3, 1×2, 7×1
            DieType::Gray => [
                Face::Flag,
                Face::Flag,
                Face::Flag,
                Face::Number(1),
                Face::Number(1),
                Face::Number(7),
            ],
        }
    }

    /// Returns true if this die type is a special-character die.
    pub fn is_special_die(self) -> bool {
        matches!(
            self,
            DieType::Minotaur | DieType::Griffin | DieType::Mermaid
        )
    }

    /// All die types in the bag.
    pub const ALL: [DieType; 7] = [
        DieType::Minotaur,
        DieType::Griffin,
        DieType::Mermaid,
        DieType::Red,
        DieType::Yellow,
        DieType::Purple,
        DieType::Gray,
    ];

    /// Total dice in the bag (36).
    pub fn total_in_bag() -> u8 {
        Self::ALL.iter().map(|d| d.bag_count()).sum()
    }
}

// ── Probability distribution for a single die ────────────────────────────────

/// A (face, probability) pair.
#[derive(Debug, Clone, Copy)]
pub struct FaceProb {
    pub face: Face,
    /// Probability as a fraction: count / 6.
    pub prob: f64,
}

/// Returns the probability distribution over distinct face outcomes for a die type.
/// Duplicate faces are merged; probabilities sum to 1.0.
pub fn face_distribution(die: DieType) -> Vec<FaceProb> {
    let faces = die.faces();
    let mut map: std::collections::HashMap<Face, u8> = std::collections::HashMap::new();
    for f in &faces {
        *map.entry(*f).or_insert(0) += 1;
    }
    let total = faces.len() as f64;
    let mut dist: Vec<FaceProb> = map
        .into_iter()
        .map(|(face, count)| FaceProb {
            face,
            prob: count as f64 / total,
        })
        .collect();
    // Deterministic order: Flag first, then numbers ascending, then specials.
    dist.sort_by(|a, b| face_order_key(a.face).cmp(&face_order_key(b.face)));
    dist
}

fn face_order_key(f: Face) -> u16 {
    match f {
        Face::Flag => 0,
        Face::Number(n) => n as u16,
        Face::Mermaid => 100,
        Face::Griffin => 101,
        Face::Minotaur => 102,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bag_total_is_36() {
        assert_eq!(DieType::total_in_bag(), 36);
    }

    #[test]
    fn all_dice_have_six_faces() {
        for die in &DieType::ALL {
            assert_eq!(die.faces().len(), 6, "{die:?} does not have 6 faces");
        }
    }

    #[test]
    fn face_distributions_sum_to_one() {
        for die in &DieType::ALL {
            let total: f64 = face_distribution(*die).iter().map(|fp| fp.prob).sum();
            assert!(
                (total - 1.0).abs() < 1e-10,
                "{die:?} probabilities sum to {total}"
            );
        }
    }

    #[test]
    fn minotaur_die_two_thirds_minotaur() {
        let dist = face_distribution(DieType::Minotaur);
        let p = dist
            .iter()
            .find(|fp| fp.face == Face::Minotaur)
            .unwrap()
            .prob;
        assert!((p - 4.0 / 6.0).abs() < 1e-10);
    }

    #[test]
    fn gray_die_half_flags() {
        let dist = face_distribution(DieType::Gray);
        let p = dist.iter().find(|fp| fp.face == Face::Flag).unwrap().prob;
        assert!((p - 3.0 / 6.0).abs() < 1e-10);
    }

    #[test]
    fn red_die_has_no_flags() {
        let dist = face_distribution(DieType::Red);
        assert!(dist.iter().all(|fp| fp.face != Face::Flag));
    }
}
