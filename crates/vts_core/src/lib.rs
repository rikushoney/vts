pub mod arch1;
pub mod design;
pub mod device;
pub mod interchange;
pub mod place;
pub mod route;
pub mod synth;

use place::Placer;
use route::Router;

pub trait Flow<PS: place::State, P: Placer<PS>, RS: route::State, R: Router<RS>> {}
