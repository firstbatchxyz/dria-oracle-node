mod handler;
pub use handler::handle_request;

mod nonce;
pub use nonce::mine_nonce;

mod generation;
pub use generation::handle_generation;

pub mod validation;
pub use validation::handle_validation;

mod utils;
use utils::parse_downloadable;
