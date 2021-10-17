export EXAMPLE_NAME=${1:-atari_antic}
export HTML_FILE=${1:-index}.html

cargo build --example $EXAMPLE_NAME --target wasm32-unknown-unknown --release;
wasm-bindgen --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/release/examples/${EXAMPLE_NAME}.wasm

envsubst <<EOF > $HTML_FILE
<html>
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  </head>
  <body>
    <script type="module">
      import init from "./web/${EXAMPLE_NAME}.js";
      window.addEventListener("load", () => {
        init();
      });
    </script>
  </body>
</html>
EOF
