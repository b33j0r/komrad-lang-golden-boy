use komrad_ast::prelude::Value;

/// This is the internal representation of the expected protocol used
/// by a delegate responder handler. These are the command implemenations
/// for the messages user code can send to the `response` that they get
/// in the the request handler pattern, e.g. `[http _response GET "about"]`
/// where response is the object having this trait's vocabulary.
trait ResponseProtocol:
    ResponseMetadataProtocol + ResponseWriteProtocol + ResponseFinalizerProtocol
{
}

// TODO: I don't know if all of these are reasonable values
pub enum CacheControl {
    NoCache,
    NoStore,
    Private,
    Public,
    MaxAge(u32),
    MustRevalidate,
    ProxyRevalidate,
    Immutable,
}

trait ResponseMetadataProtocol {
    /// Defaults to 200 if not set.
    fn set_status(&self, status: u16);

    /// Sets a cookie. Can be used multiple times.
    fn set_cookie(&self, name: String, value: String);

    /// Sets the content type. Calling multiple times will override previous values.
    fn set_content_type(&self, content_type: String);

    /// Sets an arbitrary header. Can be used multiple times.
    fn set_header(&self, name: String, value: String);

    /// Sets the cache control header. Can be used multiple times.
    fn set_cache_control(&self, cache_control: CacheControl);
}

trait ResponseWriteProtocol {
    /// Decodes and writes a komrad value to the response. Can be used multiple times.
    fn write_value(&self, value: Value);

    // For reference:

    // #[derive(Debug, Clone)]
    // pub enum Value {
    //     Empty,                      // no-op
    //     Error(RuntimeError),        // to string
    //     Channel(Channel),           // to string
    //     Boolean(bool),              // to string
    //     Word(String),               // replace with scope? not implemented yet
    //     String(String),             // utf-8
    //     Number(Number),             // to string
    //     List(Vec<Value>),           // recursive
    //     Block(Box<Block>),          // to string
    //     Bytes(Vec<u8>),             // send binary
    //     Embedded(EmbeddedBlock),    // use block.text
    // }

    //#[derive(Debug, Clone)]
    // pub enum Number {
    //     Int(literal::Int),
    //     UInt(literal::UInt),
    //     Float(literal::Float),
    // }
}

trait ResponseFinalizerProtocol {
    /// Sends the response to the client, assuming the client has set all appropriate fields.
    fn finish(&self);

    /// Sends a redirect response to the client, Calls set_status, set_location, and finish.
    fn redirect(&self, location: String);

    /// Sends immediately, overriding content-type with `text/plain`.
    fn text(&self, body: String);

    /// Sends immediately, overriding content-type with `text/html`.
    fn html(&self, body: String);

    /// Sends immediately, overriding content-type with `application/json`.
    fn json(&self, body: String);

    /// Sends immediately, overriding content-type with `application/octet-stream`.
    fn binary(&self, body: Vec<u8>);
}

// ... TODO implementation of ResponseProtocol and ResponseAgent
