import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { CSS3DObject, CSS3DRenderer } from 'three/addons/renderers/CSS3DRenderer.js';

/**
 * WebDeck 3D showcase: a studio console model with the live app
 * (same-origin iframe via CSS3DRenderer) mounted as its screen.
 *
 * Capture API (used by e2e/demo-3d.spec.ts, handy in devtools too):
 *   window.__demo3d = { ready, angles, setAngle(name), setAutoRotate(on) }
 */

export const ANGLES = ['front', 'hero', 'side', 'top', 'cover'] as const;
export type AngleName = (typeof ANGLES)[number];

interface ShowcaseApi {
  ready: boolean;
  angles: readonly string[];
  setAngle: (name: AngleName) => void;
  setAutoRotate: (on: boolean) => void;
}

declare global {
  interface Window {
    __demo3d?: ShowcaseApi;
  }
}

// Panel geometry (world units): 1280x720 iframe at 0.01 scale.
const PANEL_W = 12.8;
const PANEL_H = 7.2;
const PANEL_Y = 4.7;
const PANEL_TILT = THREE.MathUtils.degToRad(-4);

const CAMERA_POS: Record<AngleName, [number, number, number]> = {
  front: [0, 4.9, 17.5],
  hero: [11.5, 6.8, 13.5],
  side: [17, 5.2, 2.8],
  top: [0.5, 15.5, 8],
  cover: [4.5, 5.8, 16.5],
};
const TARGET: [number, number, number] = [0, 4.1, 0];

const reduceMotion =
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

function buildConsole(): { group: THREE.Group; deskParts: THREE.Object3D[] } {
  const group = new THREE.Group();
  const deskParts: THREE.Object3D[] = [];

  const charcoal = new THREE.MeshStandardMaterial({
    color: 0x1a1e24,
    roughness: 0.55,
    metalness: 0.35,
  });

  // Console body the screen stands on.
  const body = new THREE.Mesh(new RoundedBoxGeometry(14.6, 1.1, 5.2, 4, 0.12), charcoal);
  body.position.set(0, 0.55, 0.7);
  body.castShadow = true;
  body.receiveShadow = true;
  group.add(body);
  deskParts.push(body);

  // Stand neck tying the screen to the body.
  const neck = new THREE.Mesh(new RoundedBoxGeometry(3.2, 1.6, 0.7, 4, 0.1), charcoal);
  neck.position.set(0, 1.2, -0.15);
  neck.castShadow = true;
  group.add(neck);
  deskParts.push(neck);

  // Single restrained accent strip along the body's front edge.
  const strip = new THREE.Mesh(
    new THREE.BoxGeometry(13.2, 0.07, 0.07),
    new THREE.MeshStandardMaterial({
      color: 0xe8a33d,
      emissive: 0xe8a33d,
      emissiveIntensity: 1.6,
      roughness: 0.4,
    })
  );
  strip.position.set(0, 0.32, 3.32);
  group.add(strip);
  deskParts.push(strip);

  return { group, deskParts };
}

/**
 * Tablet frame around the live panel: layered titanium rim + gasket,
 * front camera, side buttons. Always visible (also the floating device
 * in clean cover mode).
 */
function buildTablet(): THREE.Group {
  const tablet = new THREE.Group();
  tablet.position.set(0, PANEL_Y, 0);
  tablet.rotation.x = PANEL_TILT;

  const titanium = new THREE.MeshStandardMaterial({
    color: 0x1e2229,
    roughness: 0.38,
    metalness: 0.85,
    envMapIntensity: 0.55,
  });
  const gasketMat = new THREE.MeshStandardMaterial({
    color: 0x07080b,
    roughness: 0.6,
    metalness: 0.2,
  });

  const rim = new THREE.Mesh(
    new RoundedBoxGeometry(PANEL_W + 1.1, PANEL_H + 1.1, 0.5, 4, 0.14),
    titanium
  );
  rim.position.z = -0.3;
  rim.castShadow = true;
  tablet.add(rim);

  const gasket = new THREE.Mesh(
    new RoundedBoxGeometry(PANEL_W + 0.45, PANEL_H + 0.45, 0.42, 4, 0.1),
    gasketMat
  );
  gasket.position.z = -0.22;
  gasket.castShadow = true;
  tablet.add(gasket);

  // Front camera: dark housing + glass lens on the top rim band.
  const camY = PANEL_H / 2 + 0.39;
  const housing = new THREE.Mesh(
    new THREE.CylinderGeometry(0.11, 0.11, 0.08, 24),
    new THREE.MeshStandardMaterial({ color: 0x05070a, roughness: 0.15, metalness: 0.6 })
  );
  housing.rotation.x = Math.PI / 2;
  housing.position.set(0, camY, -0.03);
  tablet.add(housing);
  const lens = new THREE.Mesh(
    new THREE.CylinderGeometry(0.045, 0.045, 0.09, 16),
    new THREE.MeshStandardMaterial({
      color: 0x0a1520,
      emissive: 0x16324f,
      emissiveIntensity: 0.7,
      roughness: 0.1,
      metalness: 0.4,
    })
  );
  lens.rotation.x = Math.PI / 2;
  lens.position.set(0, camY, -0.02);
  tablet.add(lens);

  // Side buttons half-embedded in the right edge.
  const edgeX = (PANEL_W + 1.1) / 2;
  const buttonMat = new THREE.MeshStandardMaterial({
    color: 0x2c313a,
    roughness: 0.4,
    metalness: 0.8,
    envMapIntensity: 0.55,
  });
  const power = new THREE.Mesh(new THREE.BoxGeometry(0.1, 0.55, 0.14), buttonMat);
  power.position.set(edgeX + 0.01, 1.6, -0.15);
  tablet.add(power);
  const volume = new THREE.Mesh(new THREE.BoxGeometry(0.1, 0.95, 0.14), buttonMat);
  volume.position.set(edgeX + 0.01, 2.65, -0.15);
  tablet.add(volume);

  return tablet;
}

function buildRoom(scene: THREE.Scene, clean: boolean): THREE.Mesh {
  const bg = clean ? 0x12141a : 0x0b0d10;
  scene.background = new THREE.Color(bg);
  scene.fog = new THREE.Fog(bg, 30, 70);

  const floor = new THREE.Mesh(
    new THREE.CircleGeometry(45, 64),
    new THREE.MeshStandardMaterial({ color: 0x14171c, roughness: 0.85, metalness: 0 })
  );
  floor.rotation.x = -Math.PI / 2;
  floor.receiveShadow = true;
  floor.visible = !clean;
  scene.add(floor);

  scene.add(new THREE.HemisphereLight(0xdfe8f2, 0x1a1d22, 0.55));

  const key = new THREE.SpotLight(0xfff1dd, 900, 60, Math.PI / 5, 0.45, 2);
  key.position.set(9, 16, 10);
  key.castShadow = !clean;
  key.shadow.mapSize.set(2048, 2048);
  key.shadow.bias = -0.0001;
  key.shadow.normalBias = 0.02;
  scene.add(key);
  key.target.position.set(0, 3, 0);
  scene.add(key.target);

  const rim = new THREE.DirectionalLight(0xbcd0e8, 1.1);
  rim.position.set(-8, 7, -9);
  scene.add(rim);

  return floor;
}

function buildLivePanel(appUrl: string): { object: CSS3DObject; iframe: HTMLIFrameElement } {
  const wrap = document.createElement('div');
  wrap.className = 'wd3d-panel';
  const iframe = document.createElement('iframe');
  iframe.src = appUrl;
  iframe.title = 'WebDeck live app';
  wrap.appendChild(iframe);

  const object = new CSS3DObject(wrap);
  // Sized in world units; the tablet group owns position/tilt.
  object.scale.setScalar(0.01);
  return { object, iframe };
}

function main(): void {
  const stage = document.getElementById('stage');
  if (!stage) return;

  const params = new URLSearchParams(window.location.search);
  const clean = params.get('clean') === '1';
  if (clean) document.body.classList.add('clean');

  const scene = new THREE.Scene();
  buildRoom(scene, clean);
  const { group: consoleGroup, deskParts } = buildConsole();
  scene.add(consoleGroup);
  if (clean) {
    // Minimalist cover framing: floating tablet only, no floor furniture.
    for (const part of deskParts) part.visible = false;
  }

  const camera = new THREE.PerspectiveCamera(40, window.innerWidth / window.innerHeight, 0.1, 200);
  camera.position.set(...CAMERA_POS.hero);
  camera.lookAt(...TARGET);

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.shadowMap.enabled = !clean;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  // Procedural studio reflections: metals need an environment map or
  // they render black. No external HDR assets required.
  const pmrem = new THREE.PMREMGenerator(renderer);
  scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  pmrem.dispose();
  stage.appendChild(renderer.domElement);

  const cssRenderer = new CSS3DRenderer();
  cssRenderer.setSize(window.innerWidth, window.innerHeight);
  // Let orbit drags pass through everywhere except the live iframe itself.
  cssRenderer.domElement.style.position = 'absolute';
  cssRenderer.domElement.style.inset = '0';
  cssRenderer.domElement.style.pointerEvents = 'none';
  stage.appendChild(cssRenderer.domElement);

  const appUrl = params.get('app') ?? '/';
  const { object: panel } = buildLivePanel(appUrl);
  const tablet = buildTablet();
  tablet.add(panel);
  scene.add(tablet);

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.target.set(...TARGET);
  controls.enableDamping = true;
  controls.dampingFactor = 0.06;
  controls.minDistance = 8;
  controls.maxDistance = 30;
  controls.minPolarAngle = THREE.MathUtils.degToRad(35);
  controls.maxPolarAngle = THREE.MathUtils.degToRad(86);
  controls.minAzimuthAngle = THREE.MathUtils.degToRad(-62);
  controls.maxAzimuthAngle = THREE.MathUtils.degToRad(62);
  controls.enablePan = false;
  controls.autoRotate = !reduceMotion;
  controls.autoRotateSpeed = 0.7;

  // Camera tween for preset angles (snaps under reduced motion).
  let tween: { from: THREE.Vector3; to: THREE.Vector3; t0: number } | null = null;
  const TWEEN_MS = 650;
  function flyTo(name: AngleName): void {
    const to = new THREE.Vector3(...CAMERA_POS[name]);
    if (reduceMotion) {
      camera.position.copy(to);
      controls.update();
      return;
    }
    tween = { from: camera.position.clone(), to, t0: performance.now() };
  }
  function stepTween(now: number): void {
    if (!tween) return;
    const k = Math.min((now - tween.t0) / TWEEN_MS, 1);
    const eased = 1 - Math.pow(1 - k, 3);
    camera.position.lerpVectors(tween.from, tween.to, eased);
    if (k >= 1) tween = null;
  }

  // HUD wiring.
  const buttons = [...document.querySelectorAll<HTMLButtonElement>('[data-angle]')];
  function markActive(name: string): void {
    for (const button of buttons) button.classList.toggle('is-active', button.dataset.angle === name);
  }
  for (const button of buttons) {
    button.addEventListener('click', () => {
      controls.autoRotate = false;
      syncRotateButton();
      flyTo(button.dataset.angle as AngleName);
      markActive(button.dataset.angle ?? '');
    });
  }
  const rotateButton = document.querySelector<HTMLButtonElement>('[data-rotate]');
  function syncRotateButton(): void {
    rotateButton?.classList.toggle('is-active', controls.autoRotate);
    rotateButton?.setAttribute('aria-pressed', String(controls.autoRotate));
  }
  rotateButton?.addEventListener('click', () => {
    controls.autoRotate = !controls.autoRotate;
    syncRotateButton();
  });
  syncRotateButton();

  const api: ShowcaseApi = {
    ready: false,
    angles: ANGLES,
    setAngle: (name: AngleName) => {
      controls.autoRotate = false;
      syncRotateButton();
      flyTo(name);
      markActive(name);
    },
    setAutoRotate: (on: boolean) => {
      controls.autoRotate = on;
      syncRotateButton();
    },
  };
  window.__demo3d = api;

  let frames = 0;
  function animate(): void {
    requestAnimationFrame(animate);
    stepTween(performance.now());
    controls.update();
    renderer.render(scene, camera);
    cssRenderer.render(scene, camera);
    frames += 1;
    if (frames === 12) api.ready = true;
  }
  animate();

  window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
    cssRenderer.setSize(window.innerWidth, window.innerHeight);
  });
}

main();
