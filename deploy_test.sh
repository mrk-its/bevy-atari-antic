set -e
set -x
if [[ -n $(git status -s | grep -v '??') ]]; then 
  echo git not clean, qutting
  exit 1
fi

DEST=$(git rev-parse --short HEAD)
mkdir $DEST

./build_wasm32_example.sh

cp -v index.html $DEST
cp -v -a web $DEST

git checkout web -f
git add $DEST
