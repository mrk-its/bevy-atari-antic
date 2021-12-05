export EXAMPLE_NAME=atari_antic
export HTML_FILE=index.html

export RUSTFLAGS=--cfg=web_sys_unstable_apis

cargo build --example $EXAMPLE_NAME --target wasm32-unknown-unknown --release --no-default-features --features webgl$*
wasm-bindgen --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/release/examples/${EXAMPLE_NAME}.wasm

envsubst <<EOF > $HTML_FILE
<html>
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
  </head>
  <body>
    <script type="module">
      import init from "./web/${EXAMPLE_NAME}.js";

      function showDebug(header, text) {
        console.log(header, text);
        let cont = document.getElementById("debug-info");
        let p = document.createElement("p");
        p.innerHTML = '<b>' + header + '</b>: ' + text;
        cont.appendChild(p);
      }

      window.addEventListener("load", () => {
        showDebug("User Agent", window.navigator.userAgent);
        let canvas = document.createElement("canvas");
        canvas.style.display = "none";
        document.body.appendChild(canvas);
        let gl = canvas.getContext('webgl2');
        if(gl) {
          let debugInfo = gl.getExtension('WEBGL_debug_renderer_info');
          let vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL);
          let renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);

          showDebug("Vendor", vendor);
          showDebug("Renderer", renderer);
        } else {
          showDebug("Error", "WEBGL2 not available")
        }
        init().catch(e => !e.toString().match("This isn't actually an error") && console.error(e));
      });
    </script>
    <div id="debug-info"></div>
  </body>
</html>
EOF
