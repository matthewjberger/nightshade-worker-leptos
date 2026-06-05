use nightshade::prelude::serde::{Deserialize, Serialize};

/// Which subject the spin and pick systems are currently driving.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "nightshade::prelude::serde")]
pub enum SubjectKind {
    #[default]
    Cube,
    Helmet,
}

/// A spinnable subject. Each subject game entity links to an engine render
/// entity through [`EngineEntity`](nightshade::prelude::EngineEntity).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(crate = "nightshade::prelude::serde")]
pub struct Subject {
    pub kind: SubjectKind,
}

/// Tags the game entity whose engine entity marks the last pick.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(crate = "nightshade::prelude::serde")]
pub struct PickMarker;
