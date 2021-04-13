fn main() {
    prost_build::compile_protos(&["src/subscribe.proto"], &["src/"]).unwrap();
}
