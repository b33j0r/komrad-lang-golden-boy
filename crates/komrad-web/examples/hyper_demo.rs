use warp::ws::WebSocket;

pub trait WebSocketLifecycle {
    type Next: WebSocketLifecycle;

    fn on_open(&mut self, ws: &WebSocket) -> Self::Next;
    fn on_message(&mut self, ws: &WebSocket, msg: String) -> Self::Next;
    fn on_close(&mut self, ws: &WebSocket) -> Self::Next;
}
