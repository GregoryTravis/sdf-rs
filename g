rm -f image00*.png out.gif out.mov anim.png
cargo run --release
open -a Google\ Chrome anim.png
# . make-anim
# open out.mov