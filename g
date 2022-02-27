rm -f image00*.png out.gif out.mov
cargo run --release
. make-anim
open out.mov