// Copyright 2022 Singularity Data
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use risingwave_common::ensure;
use risingwave_common::error::Result;
use risingwave_pb::meta::table_fragments::fragment::{FragmentDistributionType, FragmentType};
use risingwave_pb::stream_plan::StreamNode;

use crate::model::FragmentId;

/// [`StreamFragment`] represent a fragment node in fragment DAG.
#[derive(Clone, Debug)]
pub struct StreamFragment {
    /// the allocated fragment id.
    fragment_id: FragmentId,

    /// root stream node in this fragment.
    node: Arc<StreamNode>,

    /// type of this fragment.
    fragment_type: FragmentType,

    /// mark whether this fragment should only have one actor.
    is_singleton: bool,
}

impl StreamFragment {
    pub fn new(fragment_id: FragmentId, node: Arc<StreamNode>) -> Self {
        Self {
            fragment_id,
            node,
            fragment_type: FragmentType::Others,
            is_singleton: false,
        }
    }

    pub fn get_fragment_id(&self) -> FragmentId {
        self.fragment_id
    }

    pub fn get_node(&self) -> Arc<StreamNode> {
        self.node.clone()
    }

    pub fn set_fragment_type(&mut self, fragment_type: FragmentType) {
        self.fragment_type = fragment_type;
    }

    pub fn is_singleton(&self) -> bool {
        self.is_singleton
    }

    pub fn set_singleton(&mut self, is_singleton: bool) {
        self.is_singleton = is_singleton;
    }
}

/// [`StreamFragmentGraph`] stores a fragment graph with a root fragment(id: `fragment_id`).
pub struct StreamFragmentGraph {
    /// represent the root fragment of the graph.
    fragment_id: FragmentId,

    /// stores all the fragments in the graph.
    fragments: HashMap<FragmentId, StreamFragment>,

    /// stores fragment relations: parent_fragment => set(child_fragment).
    child_edges: HashMap<FragmentId, BTreeSet<FragmentId>>,
}

impl StreamFragmentGraph {
    pub fn new(fragment_id: Option<FragmentId>) -> Self {
        Self {
            fragment_id: fragment_id.unwrap_or(0),
            fragments: HashMap::new(),
            child_edges: HashMap::new(),
        }
    }

    pub fn get_root_fragment(&self) -> StreamFragment {
        self.fragments.get(&self.fragment_id).unwrap().clone()
    }

    pub fn add_fragment(&mut self, stream_fragment: StreamFragment, is_root: bool) {
        if is_root {
            self.fragment_id = stream_fragment.fragment_id;
        }
        self.fragments
            .insert(stream_fragment.fragment_id, stream_fragment);
    }

    /// Links `child_id` to its belonging parent fragment.
    pub fn link_child(&mut self, parent_id: FragmentId, child_id: FragmentId) {
        self.child_edges
            .entry(parent_id)
            .or_insert_with(BTreeSet::new)
            .insert(child_id);
    }

    pub fn has_upstream(&self, fragment_id: FragmentId) -> bool {
        self.child_edges.contains_key(&fragment_id)
    }

    pub fn get_upstream_fragments(&self, fragment_id: FragmentId) -> Option<BTreeSet<FragmentId>> {
        self.child_edges.get(&fragment_id).cloned()
    }

    pub fn get_fragment_by_id(&self, fragment_id: FragmentId) -> Option<StreamFragment> {
        self.fragments.get(&fragment_id).cloned()
    }

    pub fn get_fragment_type_by_id(&self, fragment_id: FragmentId) -> Result<FragmentType> {
        ensure!(
            self.fragments.contains_key(&fragment_id),
            "fragment id not exist!"
        );
        Ok(self.fragments.get(&fragment_id).unwrap().fragment_type)
    }

    pub fn get_distribution_type_by_id(
        &self,
        fragment_id: FragmentId,
    ) -> Result<FragmentDistributionType> {
        ensure!(
            self.fragments.contains_key(&fragment_id),
            "fragment id not exist!"
        );
        Ok(if self.fragments.get(&fragment_id).unwrap().is_singleton {
            FragmentDistributionType::Single
        } else {
            FragmentDistributionType::Hash
        })
    }
}
