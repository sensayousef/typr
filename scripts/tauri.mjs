// Wrapper around the Tauri CLI.
//
// Before invoking Tauri (which drives cargo), it resolves the Windows build
// toolchain dynamically and writes it into src-tauri/.cargo/config.toml, and
// scrubs a poisonous `WHISPER_DONT_GENERATE_BINDINGS` that would otherwise make
// whisper-rs-sys copy its bundled *Linux* bindings (-> E0080 build errors).
// All toolchain discovery lives in build-env.mjs and is version-agnostic, so
// updating Visual Studio / the Windows SDK / LLVM never breaks the build.
import { run } from "@tauri-apps/cli";
import { applyBuildEnv } from "./build-env.mjs";

try {
  const { env, wrote } = applyBuildEnv();
  if (wrote) {
    console.log(
      `[robin] build env: libclang ${env._detail.libclang.version}, ` +
        `Windows SDK ${env._detail.sdk.version} -> updated .cargo/config.toml`,
    );
  }
} catch (err) {
  console.error("\n[robin] Build environment setup failed:\n  " + err.message + "\n");
  process.exit(1);
}

run(process.argv.slice(2), "tauri").catch((err) => {
  console.error(err);
  process.exit(1);
});
