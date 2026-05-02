/// <reference lib="deno.ns" />

const ROOT = new URL("../", import.meta.url);
const METAINFO = new URL("../src-tauri/bundle/opendeck.metainfo.xml", import.meta.url);
const STARTERPACK_MANIFEST = new URL("../plugins/com.amansprojects.starterpack.sdPlugin/assets/manifest.json", import.meta.url);

function usage(): never {
	console.error("Usage: deno run -A scripts/bump_version.ts <version>");
	Deno.exit(1);
}

async function run(cmd: string, args: string[], cwd = ROOT): Promise<string> {
	const res = await new Deno.Command(cmd, { args, cwd, stdout: "piped", stderr: "piped" }).output();
	const out = new TextDecoder().decode(res.stdout).trim();
	if (res.code === 0) return out;
	const err = new TextDecoder().decode(res.stderr).trim();
	throw new Error(err || `${cmd} failed`);
}

async function bumpCrateVersion(version: string) {
	const cwd = new URL(`../src-tauri/`, import.meta.url);
	const manifest = new URL("Cargo.toml", cwd);

	const toml = await Deno.readTextFile(manifest);
	const pkgStart = toml.search(/^\[package\]\s*$/m);
	if (pkgStart < 0) throw new Error(`Failed to find [package] in Cargo.toml`);

	const nextHeader = (() => {
		const re = /^\[[^\]]+\]\s*$/gm;
		re.lastIndex = pkgStart + 1;
		const m = re.exec(toml);
		return m?.index ?? toml.length;
	})();

	const section = toml.slice(pkgStart, nextHeader);
	const updated = section.replace(/^\s*version\s*=\s*"[^"]+"\s*$/m, `version = "${version}"`);
	if (updated === section) throw new Error(`Cargo.toml already contains ${version}`);
	await Deno.writeTextFile(manifest, toml.slice(0, pkgStart) + updated + toml.slice(nextHeader));

	await run("cargo", ["update", "-p", "opendeck", "--manifest-path", manifest.pathname], cwd);
}

async function bumpStarterPackManifestVersion(version: string) {
	const json = await Deno.readTextFile(STARTERPACK_MANIFEST);
	const match = /(^\s*\"Version\"\s*:\s*)\"([^\"]+)\"/m.exec(json);
	if (!match) throw new Error("Failed to find Version in starter pack manifest.json");
	const current = match[2];
	if (current === version) throw new Error(`Starter pack manifest.json already contains ${version}`);

	const updated = json.replace(match[0], `${match[1]}"${version}"`);
	await Deno.writeTextFile(STARTERPACK_MANIFEST, updated);
}

function xmlEscape(s: string): string {
	const map: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&apos;" };
	return s.replace(/[&<>"']/g, (m) => map[m]);
}

async function prependMetainfoRelease(version: string) {
	const fromRef = await run("git", ["describe", "--tags", "--abbrev=0"]);
	const subjectsRaw = await run("git", ["log", "--reverse", "--pretty=format:%s", `${fromRef}..HEAD`]);
	const subjects = subjectsRaw.split(/\r?\n/).map((s) => s.trim()).filter(Boolean);

	const xml = await Deno.readTextFile(METAINFO);
	if (xml.includes(`version="${version}"`)) throw new Error(`opendeck.metainfo.xml already contains ${version}`);

	const li = subjects.map((s) => `\t\t\t\t\t<li>${xmlEscape(s)}</li>`).join("\n");
	const block = [
		`\t\t<release version="${version}" date="${new Date().toISOString().split("T")[0]}">`,
		`\t\t\t<url type="details">https://github.com/nekename/OpenDeck/releases/tag/v${version}</url>`,
		"\t\t\t<description>",
		"\t\t\t\t<ul>",
		li,
		"\t\t\t\t</ul>",
		"\t\t\t</description>",
		"\t\t</release>",
	].join("\n");

	const updated = xml.replace(/^\s*<releases>\s*$/m, (m) => `${m}\n${block}`);
	if (updated === xml) throw new Error("Failed to find <releases> in opendeck.metainfo.xml");
	await Deno.writeTextFile(METAINFO, updated);

	console.log(subjects.map((s) => `- ${s}`).join("\n"));
}

if (import.meta.main) {
	const version = Deno.args[0]?.trim().replace(/^v/, "") ?? usage();
	if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) usage();

	await bumpCrateVersion(version);
	await bumpStarterPackManifestVersion(version);
	await prependMetainfoRelease(version);
}
