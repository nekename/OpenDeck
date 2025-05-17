import { copy } from "jsr:@std/fs";
import { join } from "jsr:@std/path";

if (Deno.args.length < 1) Deno.exit(1);
const outDir = Deno.args[0];
try {
	Deno.remove(outDir, { recursive: true });
} catch (error: any) {
	if (!(error instanceof Deno.errors.NotFound)) {
		throw error;
	}
}
copy("files", outDir);

if (!(await new Deno.Command("cargo", { args: ["build", "--release"] }).spawn().status).success) Deno.exit(1);
const builtExec = Deno.build.os == "windows" ? "opendeck-starterpack.exe" : "opendeck-starterpack";
const targetExec = Deno.build.os == "windows" ? "opendeck-starterpack.exe" : Deno.build.os == "darwin" ? "opendeck-starterpack-mac" : "opendeck-starterpack";
copy(join("target", "release", builtExec), join(outDir, targetExec));
