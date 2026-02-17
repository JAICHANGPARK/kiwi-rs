# Troubleshooting

Map observed errors to concrete fixes. Keep fixes specific and immediately actionable.

## Error-to-Fix Table

| Symptom or error text | Likely cause | Fix |
|---|---|---|
| `failed to load library` | Dynamic library path not found or incompatible binary | Set `KIWI_LIBRARY_PATH` correctly, or use `Kiwi::init()` to bootstrap runtime assets. |
| `failed to load symbol` | Library version mismatch with expected C API symbols | Use a matching Kiwi release and re-bootstrap (`KIWI_RS_VERSION` pin if needed). |
| `bootstrap error` | Missing external commands or download/extract failure | Ensure `curl` and `tar` are available (Windows: `powershell` for archive extraction). |
| `kiwi api error` | Native call failed due to invalid input or runtime state | Reduce to minimal repro input, verify model path, and test with closest example file first. |
| `invalid argument` | Wrong option/value combination | Validate offsets, tags, span bounds, and option ranges before call. |
| UTF-16 API call not available | Loaded library lacks UTF-16 support | Gate with `supports_utf16_api()` and provide UTF-8 fallback path. |
| `analyze_many_utf16_via_native` path unavailable | Runtime lacks MW UTF-16 native support | Gate with `supports_analyze_mw()` and fallback to UTF-8 or loop-based processing. |
| Expected `kiwipiepy` helper not found (`Template`, `Stopwords`, etc.) | Not part of current `kiwi-rs` exposed surface | Offer nearest `kiwi-rs` equivalent and explicitly call out unsupported parity area. |

## Debug Sequence

1. Re-run the nearest repository example for the same API family.
2. Confirm initialization path (`init` / `new` / `from_config`) is consistent with user environment.
3. Add capability checks (`supports_utf16_api`, `supports_analyze_mw`) before optional paths.
4. Minimize options and text, then reintroduce custom settings one at a time.
