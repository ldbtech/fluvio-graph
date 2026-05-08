/**
 * Builds wallet/fluvio-card.pass/ with pass.json + required PNGs from wallet/asset-source/.
 * Run: node scripts/prepare-wallet-pass.cjs
 */
const sharp = require("sharp");
const { mkdir, writeFile, rm } = require("fs/promises");
const path = require("path");

const root = path.join(__dirname, "..");
const src = path.join(root, "wallet", "asset-source");
const out = path.join(root, "wallet", "fluvio-card.pass");

async function main() {
  await rm(out, { recursive: true }).catch(() => {});
  await mkdir(out, { recursive: true });

  const passJson = {
    formatVersion: 1,
    passTypeIdentifier: "pass.com.placeholder.fluvio",
    serialNumber: "00000000000000000000000000000001",
    teamIdentifier: "XXXXXXXXXX",
    organizationName: "FluvioMe",
    description: "FluvioMe pass · template",
    logoText: "",
    foregroundColor: "rgb(237,237,239)",
    backgroundColor: "rgb(10,10,15)",
    labelColor: "rgb(148,146,169)",
    suppressStripShine: true,
    generic: {
      primaryFields: [{ key: "name", label: "", value: "Preview name" }],
      secondaryFields: [
        { key: "tagline", label: "", value: "Your line follows here" },
        { key: "handle", label: "", value: "@you" },
      ],
      auxiliaryFields: [],
    },
  };

  await writeFile(path.join(out, "pass.json"), JSON.stringify(passJson, null, 2), "utf8");

  const icon87 = path.join(src, "icon-87.png");
  await sharp(icon87).resize(29, 29).png().toFile(path.join(out, "icon.png"));
  await sharp(icon87).resize(58, 58).png().toFile(path.join(out, "icon@2x.png"));
  await sharp(icon87).resize(87, 87).png().toFile(path.join(out, "icon@3x.png"));

  const logo = path.join(src, "logo-378.png");
  const bg = { r: 10, g: 10, b: 15 };
  const fitInside = { fit: "inside" };
  await sharp(logo)
    .ensureAlpha()
    .resize(160, 50, fitInside)
    .flatten({ background: bg })
    .png()
    .toFile(path.join(out, "logo.png"));
  await sharp(logo)
    .ensureAlpha()
    .resize(320, 100, fitInside)
    .flatten({ background: bg })
    .png()
    .toFile(path.join(out, "logo@2x.png"));
  await sharp(logo)
    .ensureAlpha()
    .resize(480, 150, fitInside)
    .flatten({ background: bg })
    .png()
    .toFile(path.join(out, "logo@3x.png"));

  console.warn("Prepared", out);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
