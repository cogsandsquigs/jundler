# RUN: sh -ea @file @tmpdir

# configuration

NAME=simple

# script. DON'T TOUCH (to configure!)!

TMPPATH=$1/$NAME-$RANDOM

mkdir -p $TMPPATH
cp -R tests/fixtures/$NAME/. $TMPPATH

cargo run -- $TMPPATH

# Ok to touch!

# CHECK: Hello, world!
# CHECK: 1 + 2 = 3
$TMPPATH/$NAME
