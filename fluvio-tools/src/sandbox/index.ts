// src/sandbox/index.ts
// Fluvio Tools Sandbox — browser UI for testing furniture generators.
// Wires Three.js scene + controls panel + stats output.

import * as THREE from "three"
import { OrbitControls } from "three/addons/controls/OrbitControls.js"
import { MaterialLibrary, MaterialKey } from "../tools/MaterialLibrary"
import { generateSofa } from "../tools/sofa"
import { generateBed } from "../tools/bed"
import { generateTable } from "../tools/table"
import { generateChair } from "../tools/chair"
import { generateDesk } from "../tools/desk"

// ── Tool registry ─────────────────────────────────────────────────────────────

interface ToolDef {
  label:    string
  fn:       (style: string, material: MaterialKey) => THREE.Group
  styles:   string[]
  controls: ControlDef[]
}

interface ControlDef {
  id:    string
  label: string
  type:  "range" | "select"
  min?:  number
  max?:  number
  step?: number
  value: number | string
  options?: string[]
}

const TOOLS: Record<string, ToolDef> = {
  sofa: {
    label: "Sofa",
    fn: generateSofa,
    styles: ["modern", "scandinavian", "industrial", "curved"],
    controls: [
      { id:"width",  label:"Width (m)",  type:"range", min:1.2, max:3.5, step:0.1, value:2.2 },
      { id:"depth",  label:"Depth (m)",  type:"range", min:0.6, max:1.2, step:0.1, value:0.9 },
      { id:"height", label:"Height (m)", type:"range", min:0.6, max:1.0, step:0.05, value:0.75 },
    ],
  },
  bed: {
    label: "Bed",
    fn: generateBed,
    styles: ["modern", "scandinavian", "platform", "upholstered"],
    controls: [
      { id:"size",   label:"Size",       type:"select", value:"double",
        options:["single","double","queen","king"] },
      { id:"height", label:"Height (m)", type:"range", min:0.3, max:0.7, step:0.05, value:0.45 },
    ],
  },
  table: {
    label: "Table",
    fn: generateTable,
    styles: ["dining", "coffee", "side", "console"],
    controls: [
      { id:"width",  label:"Width (m)",  type:"range", min:0.5, max:3.0, step:0.1, value:1.8 },
      { id:"depth",  label:"Depth (m)",  type:"range", min:0.4, max:1.2, step:0.1, value:0.9 },
      { id:"height", label:"Height (m)", type:"range", min:0.35, max:0.85, step:0.05, value:0.75 },
    ],
  },
  chair: {
    label: "Chair",
    fn: generateChair,
    styles: ["dining", "lounge", "accent", "bar"],
    controls: [
      { id:"width",  label:"Width (m)",  type:"range", min:0.4, max:0.8, step:0.05, value:0.55 },
      { id:"height", label:"Height (m)", type:"range", min:0.75, max:1.1, step:0.05, value:0.85 },
    ],
  },
  desk: {
    label: "Desk",
    fn: generateDesk,
    styles: ["minimal", "executive", "corner", "standing"],
    controls: [
      { id:"width",  label:"Width (m)",  type:"range", min:0.8, max:2.2, step:0.1, value:1.4 },
      { id:"depth",  label:"Depth (m)",  type:"range", min:0.5, max:0.9, step:0.05, value:0.65 },
    ],
  },
}

// ── State ─────────────────────────────────────────────────────────────────────

let activeTool   = "sofa"
let activeMat: MaterialKey = "fabric_grey"
let activeStyle  = "modern"
let rotSpeed     = 0.005
let wireframe    = false
let currentGroup: THREE.Group | null = null

// ── Three.js setup ────────────────────────────────────────────────────────────

const wrap    = document.getElementById("canvas-wrap")!
const w       = wrap.clientWidth
const h       = wrap.clientHeight

const renderer = new THREE.WebGLRenderer({ antialias: true })
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
renderer.setSize(w, h)
renderer.shadowMap.enabled = true
renderer.shadowMap.type    = THREE.PCFSoftShadowMap
renderer.outputColorSpace  = THREE.SRGBColorSpace
wrap.appendChild(renderer.domElement)

const scene  = new THREE.Scene()
scene.background = new THREE.Color(0x0a0a10)

const camera = new THREE.PerspectiveCamera(50, w / h, 0.1, 100)
camera.position.set(2.5, 2.0, 3.5)

const controls = new OrbitControls(camera, renderer.domElement)
controls.enableDamping = true
controls.dampingFactor = 0.08
controls.minDistance   = 0.5
controls.maxDistance   = 20

// Lighting
const ambLight  = new THREE.AmbientLight(0xffffff, 0.3)
const keyLight  = new THREE.DirectionalLight(0xfff2df, 1.4)
const fillLight = new THREE.DirectionalLight(0xcad8ff, 0.5)
const rimLight  = new THREE.DirectionalLight(0xffffff, 0.3)

keyLight.position.set(4, 6, 5)
keyLight.castShadow = true
keyLight.shadow.mapSize.set(2048, 2048)
keyLight.shadow.camera.near = 0.5
keyLight.shadow.camera.far  = 30
keyLight.shadow.bias        = -0.0005

fillLight.position.set(-5, 3, -2)
rimLight.position.set(0, 4, -6)

scene.add(ambLight, keyLight, fillLight, rimLight)

// Floor
const floor = new THREE.Mesh(
  new THREE.PlaneGeometry(16, 16),
  new THREE.MeshStandardMaterial({ color: 0x1a1a22, roughness: 0.95 })
)
floor.rotation.x = -Math.PI / 2
floor.position.y = -0.01
floor.receiveShadow = true
scene.add(floor)

// Grid
const grid = new THREE.GridHelper(16, 32, 0x222233, 0x1a1a28)
scene.add(grid)

// ── Build a tool ──────────────────────────────────────────────────────────────

function buildTool() {
  document.getElementById("loading-overlay")!.classList.add("visible")

  setTimeout(() => {
    if (currentGroup) {
      scene.remove(currentGroup)
      currentGroup.traverse(o => {
        if (o instanceof THREE.Mesh) {
          o.geometry.dispose()
          if (Array.isArray(o.material)) o.material.forEach(m => m.dispose())
          else o.material.dispose()
        }
      })
    }

    const tool = TOOLS[activeTool]
    const group = tool.fn(activeStyle, activeMat)

    // Center on floor
    const box = new THREE.Box3().setFromObject(group)
    const center = box.getCenter(new THREE.Vector3())
    group.position.x -= center.x
    group.position.z -= center.z
    group.position.y -= box.min.y

    // Apply wireframe if active
    if (wireframe) {
      group.traverse(o => {
        if (o instanceof THREE.Mesh) {
          if (!Array.isArray(o.material)) o.material.wireframe = true
        }
      })
    }

    scene.add(group)
    currentGroup = group

    updateStats(group, box)
    updateJsonOut(group)

    document.getElementById("loading-overlay")!.classList.remove("visible")
  }, 50)
}

// ── Stats ─────────────────────────────────────────────────────────────────────

function updateStats(group: THREE.Group, box: THREE.Box3) {
  let polys = 0
  let objs  = 0
  group.traverse(o => {
    if (o instanceof THREE.Mesh) {
      objs++
      const geo = o.geometry
      if (geo.index) polys += geo.index.count / 3
      else polys += (geo.attributes.position?.count ?? 0) / 3
    }
  })
  const size = box.getSize(new THREE.Vector3())
  document.getElementById("s-polys")!.textContent = polys.toFixed(0)
  document.getElementById("s-objs")!.textContent  = objs.toString()
  document.getElementById("s-w")!.textContent     = size.x.toFixed(1) + "m"
  document.getElementById("s-h")!.textContent     = size.y.toFixed(1) + "m"
  document.getElementById("poly-count")!.textContent = polys.toFixed(0) + " triangles"
  document.getElementById("obj-count")!.textContent  = objs + " objects"
}

function updateJsonOut(group: THREE.Group) {
  const components: object[] = []
  group.traverse(o => {
    if (o instanceof THREE.Mesh) {
      const box = new THREE.Box3().setFromObject(o)
      const size = box.getSize(new THREE.Vector3())
      components.push({
        name:       o.name || "component",
        dimensions: [+size.x.toFixed(2), +size.y.toFixed(2), +size.z.toFixed(2)],
        position:   [+o.position.x.toFixed(3), +o.position.y.toFixed(3), +o.position.z.toFixed(3)],
      })
    }
  })

  const spec = {
    tool:       activeTool,
    style:      activeStyle,
    material:   activeMat,
    components,
  }

  const el = document.getElementById("json-out")!
  el.innerHTML = syntaxHighlight(JSON.stringify(spec, null, 2))
}

function syntaxHighlight(json: string): string {
  return json
    .replace(/("[\w]+")\s*:/g, '<span class="json-key">$1</span>:')
    .replace(/:\s*(".*?")/g,   ': <span class="json-str">$1</span>')
    .replace(/:\s*(-?\d+\.?\d*)/g, ': <span class="json-num">$1</span>')
    .replace(/:\s*(true|false)/g,  ': <span class="json-bool">$1</span>')
}

// ── Lighting presets ──────────────────────────────────────────────────────────

const envPresets: Record<string, { bg: number; amb: number; key: number; fill: number }> = {
  dark:   { bg: 0x0a0a10, amb: 0.20, key: 1.4, fill: 0.4 },
  warm:   { bg: 0x0f0d0a, amb: 0.35, key: 1.8, fill: 0.3 },
  cool:   { bg: 0x0a0c10, amb: 0.40, key: 1.2, fill: 0.6 },
  bright: { bg: 0x141420, amb: 0.60, key: 2.0, fill: 0.8 },
}

function applyEnv(preset: string) {
  const p = envPresets[preset]
  scene.background = new THREE.Color(p.bg)
  ambLight.intensity  = p.amb
  keyLight.intensity  = p.key
  fillLight.intensity = p.fill
}

// ── UI builder ────────────────────────────────────────────────────────────────

function buildTabs() {
  const el = document.getElementById("tool-tabs")!
  el.innerHTML = Object.entries(TOOLS).map(([key, def]) => `
    <button class="tool-tab ${key === activeTool ? "active" : ""}" 
            data-tool="${key}">${def.label}</button>
  `).join("")

  el.querySelectorAll(".tool-tab").forEach(btn => {
    btn.addEventListener("click", () => {
      activeTool  = (btn as HTMLElement).dataset.tool!
      activeStyle = TOOLS[activeTool].styles[0]
      buildTabs()
      buildToolControls()
      buildTool()
    })
  })
}

function buildToolControls() {
  const tool = TOOLS[activeTool]
  const el   = document.getElementById("tool-controls")!

  el.innerHTML = `
    <div class="ctrl-section">
      <div class="ctrl-label">STYLE</div>
      <select id="style-select">
        ${tool.styles.map(s => `<option value="${s}" ${s===activeStyle?"selected":""}>${s}</option>`).join("")}
      </select>
    </div>
    <div class="ctrl-section">
      <div class="ctrl-label">DIMENSIONS</div>
      ${tool.controls.map(c => {
        if (c.type === "range") return `
          <div class="ctrl-row">
            <label>${c.label} <span id="val-${c.id}">${c.value}</span></label>
            <input type="range" id="ctrl-${c.id}" 
                   min="${c.min}" max="${c.max}" step="${c.step}" value="${c.value}">
          </div>`
        if (c.type === "select") return `
          <div class="ctrl-section">
            <div class="ctrl-label">${c.label.toUpperCase()}</div>
            <select id="ctrl-${c.id}">
              ${(c.options||[]).map(o=>`<option value="${o}" ${o===c.value?"selected":""}>${o}</option>`).join("")}
            </select>
          </div>`
        return ""
      }).join("")}
    </div>
  `

  document.getElementById("style-select")!.addEventListener("change", e => {
    activeStyle = (e.target as HTMLSelectElement).value
    buildTool()
  })

  tool.controls.forEach(c => {
    const input = document.getElementById(`ctrl-${c.id}`)
    if (!input) return
    input.addEventListener("input", e => {
      const val = (e.target as HTMLInputElement).value
      const out = document.getElementById(`val-${c.id}`)
      if (out) out.textContent = val
    })
    input.addEventListener("change", () => buildTool())
  })
}

function buildMatSelect() {
  const sel = document.getElementById("mat-select")!
  sel.innerHTML = Object.keys(MaterialLibrary).map(k =>
    `<option value="${k}" ${k===activeMat?"selected":""}>${k.replace(/_/g," ")}</option>`
  ).join("")
  sel.addEventListener("change", e => {
    activeMat = (e.target as HTMLSelectElement).value as MaterialKey
    buildTool()
    buildMatChips()
  })
}

function buildMatChips() {
  const el = document.getElementById("mat-chips")!
  const COLORS: Record<string, string> = {
    white_oak:"#c8a87a", dark_walnut:"#4a3728", pine:"#d4a96a",
    polished_concrete:"#9a9a9a", raw_concrete:"#7a7a7a", marble:"#f0ece4",
    slate:"#4a4e54", terracotta:"#c2714f", fabric_grey:"#8a8a8a",
    fabric_cream:"#e8dcc8", fabric_navy:"#2a3a5a", brushed_brass:"#b8942a",
    brushed_steel:"#9a9ea8", matte_black:"#1a1a1a", glass:"#88bbff",
  }
  el.innerHTML = Object.keys(MaterialLibrary).map(k => `
    <div class="mat-chip ${k===activeMat?"active":""}" data-mat="${k}">
      <div class="mat-dot" style="background:${COLORS[k]||'#666'}"></div>
      ${k.replace(/_/g," ")}
    </div>
  `).join("")

  el.querySelectorAll(".mat-chip").forEach(chip => {
    chip.addEventListener("click", () => {
      activeMat = (chip as HTMLElement).dataset.mat as MaterialKey
      ;(document.getElementById("mat-select") as HTMLSelectElement).value = activeMat
      buildMatChips()
      buildTool()
    })
  })
}

// ── Controls wiring ───────────────────────────────────────────────────────────

document.getElementById("rot-speed")!.addEventListener("input", e => {
  rotSpeed = parseFloat((e.target as HTMLInputElement).value) * 0.01
  document.getElementById("rot-val")!.textContent =
    parseFloat((e.target as HTMLInputElement).value).toFixed(1)
})

document.getElementById("light-intensity")!.addEventListener("input", e => {
  const v = parseFloat((e.target as HTMLInputElement).value)
  keyLight.intensity = v
  document.getElementById("light-val")!.textContent = v.toFixed(1)
})

document.getElementById("env-select")!.addEventListener("change", e => {
  applyEnv((e.target as HTMLSelectElement).value)
})

document.getElementById("btn-rebuild")!.addEventListener("click", buildTool)

document.getElementById("btn-reset-cam")!.addEventListener("click", () => {
  camera.position.set(2.5, 2.0, 3.5)
  controls.target.set(0, 0.5, 0)
  controls.update()
})

document.getElementById("btn-wireframe")!.addEventListener("click", () => {
  wireframe = !wireframe
  currentGroup?.traverse(o => {
    if (o instanceof THREE.Mesh && !Array.isArray(o.material)) {
      o.material.wireframe = wireframe
    }
  })
})

document.getElementById("btn-export")!.addEventListener("click", () => {
  const el   = document.getElementById("json-out")!
  const text = el.textContent || ""
  const blob = new Blob([text], { type: "application/json" })
  const url  = URL.createObjectURL(blob)
  const a    = document.createElement("a")
  a.href     = url
  a.download = `${activeTool}_${activeStyle}_${activeMat}.json`
  a.click()
  URL.revokeObjectURL(url)
})

// ── Resize ────────────────────────────────────────────────────────────────────

window.addEventListener("resize", () => {
  const w = wrap.clientWidth
  const h = wrap.clientHeight
  camera.aspect = w / h
  camera.updateProjectionMatrix()
  renderer.setSize(w, h)
})

// ── Render loop ───────────────────────────────────────────────────────────────

renderer.setAnimationLoop(() => {
  controls.update()
  if (currentGroup) currentGroup.rotation.y += rotSpeed
  renderer.render(scene, camera)
})

// ── Init ──────────────────────────────────────────────────────────────────────

buildTabs()
buildToolControls()
buildMatSelect()
buildMatChips()
buildTool()