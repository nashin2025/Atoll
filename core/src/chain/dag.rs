//! DAG structure with GHOSTDAG fork choice for the Atoll Protocol.
//!
//! Implements a block DAG where each block can have multiple parents,
//! with GHOSTDAG (Greedy Heaviest Observed Subtree DAG) for fork choice.

use crate::primitives::hash::Hash512;
use std::collections::{HashMap, HashSet};

/// A vertex in the DAG (representing a block)
#[derive(Debug, Clone)]
pub struct DAGVertex {
    pub hash: Hash512,
    pub parents: Vec<Hash512>,
    pub height: u64,
    pub blue_score: u64,    // GHOSTDAG blue score
    pub blue_work: u64,     // GHOSTDAG blue work
    pub is_blue: bool,      // Whether this block is in the blue set
    pub anticone_size: u32, // Size of anticone (blocks conflicting with this one)
}

/// DAG structure with GHOSTDAG fork choice
pub struct DAG {
    /// Vertices stored by hash
    vertices: HashMap<Hash512, DAGVertex>,
    /// Tips (blocks with no children yet)
    tips: HashSet<Hash512>,
    /// Selected tip (the one we're building on)
    selected_tip: Option<Hash512>,
    /// K parameter (max anticone size for GHOSTDAG)
    k: u32,
}

impl DAG {
    /// Creates a new DAG with genesis vertex
    pub fn new(genesis_hash: Hash512, k: u32) -> Self {
        let mut vertices = HashMap::new();
        let mut tips = HashSet::new();

        let genesis = DAGVertex {
            hash: genesis_hash,
            parents: Vec::new(),
            height: 0,
            blue_score: 1,
            blue_work: 1,
            is_blue: true,
            anticone_size: 0,
        };

        vertices.insert(genesis_hash, genesis.clone());
        tips.insert(genesis_hash);

        Self {
            vertices,
            tips,
            selected_tip: Some(genesis_hash),
            k,
        }
    }

    /// Adds a new vertex to the DAG
    pub fn add_vertex(
        &mut self,
        hash: Hash512,
        parents: Vec<Hash512>,
        height: u64,
    ) -> Result<(), String> {
        // Verify all parents exist
        for parent in &parents {
            if !self.vertices.contains_key(parent) {
                return Err(format!("Parent {:?} not found in DAG", parent));
            }
        }

        // Calculate blue score and work
        let (blue_score, blue_work, is_blue) = self.calculate_ghostdag_scores(&parents);

        let anticone_size = self.calculate_anticone_size(&hash, &parents);

        let vertex = DAGVertex {
            hash,
            parents: parents.clone(),
            height,
            blue_score,
            blue_work,
            is_blue,
            anticone_size,
        };

        self.vertices.insert(hash, vertex.clone());

        // Update tips: remove parents that are no longer tips, add new vertex
        for parent in &parents {
            self.tips.remove(parent);
        }
        self.tips.insert(hash);

        // Update selected tip if this has higher blue work
        if let Some(selected) = self.selected_tip {
            if let Some(selected_vertex) = self.vertices.get(&selected) {
                if blue_work > selected_vertex.blue_work {
                    self.selected_tip = Some(hash);
                }
            }
        } else {
            self.selected_tip = Some(hash);
        }

        Ok(())
    }

    /// Returns the selected tip (highest blue work)
    pub fn selected_tip(&self) -> Option<Hash512> {
        self.selected_tip
    }

    /// Returns all current tips
    pub fn tips(&self) -> &HashSet<Hash512> {
        &self.tips
    }

    /// Returns a vertex by hash
    pub fn get_vertex(&self, hash: &Hash512) -> Option<&DAGVertex> {
        self.vertices.get(hash)
    }

    /// Returns the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the K parameter
    pub fn k(&self) -> u32 {
        self.k
    }

    /// Calculates GHOSTDAG blue score and work for a new vertex
    fn calculate_ghostdag_scores(&self, parents: &[Hash512]) -> (u64, u64, bool) {
        if parents.is_empty() {
            return (1, 1, true);
        }

        // Blue score = 1 + max blue score of blue parents
        let mut max_blue_score: u64 = 0;
        let mut max_blue_work: u64 = 0;
        let mut has_blue_parent = false;

        for parent in parents {
            if let Some(parent_vertex) = self.vertices.get(parent) {
                if parent_vertex.is_blue {
                    has_blue_parent = true;
                    max_blue_score = max_blue_score.max(parent_vertex.blue_score);
                    max_blue_work = max_blue_work.max(parent_vertex.blue_work);
                }
            }
        }

        if has_blue_parent {
            (max_blue_score + 1, max_blue_work + 1, true)
        } else {
            // No blue parents - this block is red (not in the blue set)
            let best_work = parents
                .iter()
                .filter_map(|p| self.vertices.get(p))
                .map(|v| v.blue_work)
                .max()
                .unwrap_or(0);
            (1, best_work, false)
        }
    }

    /// Calculates anticone size for a new vertex
    fn calculate_anticone_size(&self, _hash: &Hash512, _parents: &[Hash512]) -> u32 {
        // Phase 1: Simplified anticone calculation
        // Phase 2: Full anticone calculation with topological ordering
        0
    }

    /// Returns the blue set (all blue blocks in topological order)
    pub fn blue_set(&self) -> Vec<Hash512> {
        let mut blue: Vec<Hash512> = self
            .vertices
            .values()
            .filter(|v| v.is_blue)
            .map(|v| v.hash)
            .collect();

        // Sort by height (topological order)
        blue.sort_by(|a, b| {
            let ha = self.vertices.get(a).map(|v| v.height).unwrap_or(0);
            let hb = self.vertices.get(b).map(|v| v.height).unwrap_or(0);
            ha.cmp(&hb)
        });

        blue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_initialization() {
        let genesis = Hash512::zero();
        let dag = DAG::new(genesis, 10);
        assert_eq!(dag.vertex_count(), 1);
        assert_eq!(dag.selected_tip(), Some(genesis));
        assert!(dag.tips().contains(&genesis));
    }

    #[test]
    fn add_vertex_updates_tips() {
        let genesis = Hash512::zero();
        let mut dag = DAG::new(genesis, 10);

        let block1 = Hash512::from_bytes(&[1u8; 64]);
        dag.add_vertex(block1, vec![genesis], 1).unwrap();

        assert!(!dag.tips().contains(&genesis));
        assert!(dag.tips().contains(&block1));
        assert_eq!(dag.selected_tip(), Some(block1));
    }

    #[test]
    fn multiple_parents() {
        let genesis = Hash512::zero();
        let mut dag = DAG::new(genesis, 10);

        let block1 = Hash512::from_bytes(&[1u8; 64]);
        dag.add_vertex(block1, vec![genesis], 1).unwrap();

        let block2 = Hash512::from_bytes(&[2u8; 64]);
        dag.add_vertex(block2, vec![genesis, block1], 2).unwrap();

        assert_eq!(dag.vertex_count(), 3);
        let vertex = dag.get_vertex(&block2).unwrap();
        assert_eq!(vertex.parents.len(), 2);
        assert!(vertex.is_blue);
    }

    #[test]
    fn ghostdag_fork_choice() {
        let genesis = Hash512::zero();
        let mut dag = DAG::new(genesis, 10);

        // Build chain: genesis -> b1 -> b2 -> b3
        let b1 = Hash512::from_bytes(&[1u8; 64]);
        dag.add_vertex(b1, vec![genesis], 1).unwrap();

        let b2 = Hash512::from_bytes(&[2u8; 64]);
        dag.add_vertex(b2, vec![b1], 2).unwrap();

        let b3 = Hash512::from_bytes(&[3u8; 64]);
        dag.add_vertex(b3, vec![b2], 3).unwrap();

        // Build fork: genesis -> f1 -> f2 (shorter)
        let f1 = Hash512::from_bytes(&[4u8; 64]);
        dag.add_vertex(f1, vec![genesis], 1).unwrap();

        let f2 = Hash512::from_bytes(&[5u8; 64]);
        dag.add_vertex(f2, vec![f1], 2).unwrap();

        // b3 should be selected (higher blue work)
        assert_eq!(dag.selected_tip(), Some(b3));
    }

    #[test]
    fn blue_set_ordering() {
        let genesis = Hash512::zero();
        let mut dag = DAG::new(genesis, 10);

        let b1 = Hash512::from_bytes(&[1u8; 64]);
        dag.add_vertex(b1, vec![genesis], 1).unwrap();

        let b2 = Hash512::from_bytes(&[2u8; 64]);
        dag.add_vertex(b2, vec![b1], 2).unwrap();

        let blue = dag.blue_set();
        assert_eq!(blue.len(), 3); // genesis, b1, b2 are all blue
        assert_eq!(blue[0], genesis);
        assert_eq!(blue[1], b1);
        assert_eq!(blue[2], b2);
    }

    #[test]
    fn parent_not_found_error() {
        let genesis = Hash512::zero();
        let mut dag = DAG::new(genesis, 10);

        // Try to add a vertex with a completely unknown parent
        let unknown_parent = Hash512::from_bytes(&[88u8; 64]);
        let unknown_child = Hash512::from_bytes(&[99u8; 64]);
        let result = dag.add_vertex(unknown_child, vec![unknown_parent], 1);
        assert!(result.is_err());
    }
}
