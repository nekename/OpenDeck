/// <reference lib="deno.ns" />

import { copy } from "jsr:@std/fs";
import { join } from "jsr:@std/path";

if (Deno.args.length < 2) Deno.exit(1);
const outDir = Deno.args[0];
const target = Deno.args[1];

try {
	await Deno.remove(outDir, { recursive: true });
} catch (error: any) {
	if (!(error instanceof Deno.errors.NotFound)) {
		throw error;
	}
}

copy("assets", outDir);
if (!(await new Deno.Command("cargo", { args: ["install", "--path", ".", "--target", target, "--root", join(outDir, Deno.build.os)] }).spawn().status).success) Deno.exit(1);
