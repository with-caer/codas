Web APIs for Codas.

## Getting Started

First, run `build_web.sh` from the root of the repository to
obtain the release artifacts (`codas_web.js` and `codas_web.wasm`),
and copy these artifacts into your project.

Then, in your HTML, add:

```html
<script src="path/to/codas_web.js"></script>
<script src="index.js" type="module"></script>
```

And in your `index.js` (or similar file), add:

```js
const { encrypt_str, decrypt_str } = wasm_bindgen;
await wasm_bindgen('path/to/codas_web.wasm');
```

Replacing (or extending) `encrypt_str` and `decrypt_str`
with any of the functions exported by [`lib.rs`](src/lib.rs).

## License

Copyright 2025 With Caer, LLC.

Licensed under the MIT license. Refer to [the license file](../LICENSE.txt) for more info.