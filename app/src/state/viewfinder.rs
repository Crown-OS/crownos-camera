/// What the viewfinder area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Viewfinder {
    /// The camera is opening and has not sent a frame yet.
    Starting,
    Live,
    /// No camera sent pictures, or it failed or went away, for the reason given.
    Unavailable(String),
}
