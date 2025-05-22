> This crate is _unstable_ and experimental.

C-style dynamic libraries for Codas.

## Building for: WASM

1. Run `./ops/build_web.sh` from the root of the repository.

2. Copy `target/web/codas*` into your project.

3. In your `index.html` (or similar), add:

   ```html
   <script src="path/to/codas_web.js"></script>
   <script src="index.js" type="module"></script>
   ```

4. In your `index.js` (or similar), add:

   ```js
   const { * } = wasm_bindgen;
   await wasm_bindgen('path/to/codas_web.wasm');
   ```

   Replacing `*` with any of the functions exported by [`lib.rs`](src/lib.rs).

## Building for: Python

1. Run `./ops/build_python.sh` from the root of the repository.

## License

Copyright © 2025 With Caer, LLC.

Licensed under the MIT license. Refer to [the license file](../LICENSE.txt) for more info.