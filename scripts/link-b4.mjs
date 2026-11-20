import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "third_party", "b4-linked");
rmSync(out, { recursive: true, force: true });

for (const name of ["tokens", "motion", "react"]) {
  const src = join(root, "vendor", "b4", "packages", name);
  const dest = join(out, name);
  mkdirSync(dest, { recursive: true });
  cpSync(join(src, "dist"), join(dest, "dist"), { recursive: true });
  const pkg = JSON.parse(readFileSync(join(src, "package.json"), "utf8"));
  delete pkg.devDependencies;
  delete pkg.scripts;
  for (const [dep, version] of Object.entries(pkg.dependencies ?? {})) {
    if (version === "workspace:*") {
      pkg.dependencies[dep] = `file:../${dep.slice("@b4/".length)}`;
    }
  }
  writeFileSync(join(dest, "package.json"), `${JSON.stringify(pkg, null, 2)}\n`);
}

console.log(`Linked B4 packages into ${out}`);
