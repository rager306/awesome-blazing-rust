//! Domain: pure model, rules, FSM, projections.
//!
//! Nothing here touches the filesystem or argv. The model is the single
//! source of truth (the "metaprompt"); every artifact is a deterministic
//! projection of it; the FSM makes every pipeline step a total transition.

pub mod fsm;
pub mod model;
pub mod projection;
pub mod rules;
pub mod upstream;
