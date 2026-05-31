<script>
  import { T, useTask, useThrelte } from '@threlte/core';
  import { Float, ContactShadows, OrbitControls } from '@threlte/extras';
  import { spring } from 'svelte/motion';
  import * as THREE from 'three';
  import { dipralixEvents } from '../../lib/events';

  const { pointer } = useThrelte();
  let rotation = 0;
  let particles = Array.from({ length: 100 }, () => ({
    position: [Math.random() * 10 - 5, Math.random() * 10 - 5, Math.random() * 10 - 5],
    speed: Math.random() * 0.02,
    color: '#6366f1'
  }));

  const anvilScale = spring(1);
  const anvilColor = spring(0); // 0 = blue, 1 = white/glow

  let sparks = [];

  dipralixEvents.subscribe(events => {
    if (events.length === 0) return;
    const lastEvent = events[events.length - 1];
    
    if (lastEvent.type === 'input') {
      anvilScale.set(1.2).then(() => anvilScale.set(1));
    } else if (lastEvent.type === 'process') {
      anvilColor.set(1).then(() => anvilColor.set(0));
      // Add a burst of particles
      for (let i = 0; i < 20; i++) {
        sparks = [...sparks, {
          position: [0, 0, 0],
          velocity: [(Math.random() - 0.5) * 0.2, Math.random() * 0.2, (Math.random() - 0.5) * 0.2],
          life: 1.0,
          color: '#ffffff'
        }];
      }
    } else if (lastEvent.type === 'spark') {
       for (let i = 0; i < 10; i++) {
        sparks = [...sparks, {
          position: [0, 0, 0],
          velocity: [(Math.random() - 0.5) * 0.1, Math.random() * 0.3, (Math.random() - 0.5) * 0.1],
          life: 1.0,
          color: lastEvent.data.color || '#6366f1'
        }];
      }
    }
  });

  useTask((delta) => {
    rotation += delta * 0.2;
    particles = particles.map(p => {
      let y = p.position[1] + p.speed;
      if (y > 5) y = -5;
      return {
        ...p,
        position: [p.position[0], y, p.position[2]]
      };
    });

    sparks = sparks
      .map(s => ({
        ...s,
        position: [
          s.position[0] + s.velocity[0],
          s.position[1] + s.velocity[1],
          s.position[2] + s.velocity[2]
        ],
        velocity: [s.velocity[0], s.velocity[1] - 0.005, s.velocity[2]], // gravity
        life: s.life - 0.02
      }))
      .filter(s => s.life > 0);
  });

  const tools = [
    { name: 'read_file', color: '#6366f1', position: [2, 1, 0] },
    { name: 'write_file', color: '#22d3ee', position: [-2, 1.5, 1] },
    { name: 'bash', color: '#4ade80', position: [0, 2, -2] },
    { name: 'search', color: '#f97316', position: [1.5, -1, 2] },
    { name: 'git', color: '#a855f7', position: [-1.5, -1.5, -1.5] }
  ];
</script>

<T.PerspectiveCamera
  makeDefault
  position={[10, 5, 10]}
  fov={25}
>
  <OrbitControls
    enableZoom={false}
    autoRotate
    autoRotateSpeed={0.5}
  />
</T.PerspectiveCamera>

<T.DirectionalLight
  position={[3, 10, 10]}
  intensity={2}
/>
<T.PointLight
  position={[-10, 10, -10]}
  intensity={1}
  color="#6366f1"
/>
<T.AmbientLight intensity={0.2} />

<!-- The Anvil / Center Piece -->
<Float
  speed={2}
  rotationIntensity={0.5}
  floatIntensity={0.5}
>
  <T.Mesh 
    position={[0, 0, 0]}
    rotation={[$pointer.y * 0.5, $pointer.x * 0.5, 0]}
    scale={$anvilScale}
  >
    <T.IcosahedronGeometry args={[1, 0]} />
    <T.MeshStandardMaterial
      color="#1e1e2e"
      metalness={0.8}
      roughness={0.2}
      wireframe
    />
  </T.Mesh>
  
  <T.Mesh position={[0, 0, 0]} scale={$anvilScale * 0.9}>
    <T.IcosahedronGeometry args={[1, 0]} />
    <T.MeshStandardMaterial
      color={$anvilColor > 0.5 ? '#ffffff' : '#6366f1'}
      emissive={$anvilColor > 0.5 ? '#ffffff' : '#6366f1'}
      emissiveIntensity={2 + $anvilColor * 5}
      transparent
      opacity={0.3 + $anvilColor * 0.4}
    />
  </T.Mesh>
</Float>

<!-- Floating Tool Shapes -->
{#each tools as tool, i}
  <Float
    speed={1.5 + i * 0.2}
    rotationIntensity={1}
    floatIntensity={1}
  >
    <T.Group position={tool.position}>
      <T.Mesh>
        <T.TetrahedronGeometry args={[0.4, 0]} />
        <T.MeshStandardMaterial
          color={tool.color}
          emissive={tool.color}
          emissiveIntensity={1}
          metalness={0.9}
        />
      </T.Mesh>
      <T.PointLight
        color={tool.color}
        intensity={0.5}
        distance={3}
      />
    </T.Group>
  </Float>
{/each}

<ContactShadows
  scale={10}
  blur={2}
  far={2.5}
  opacity={0.5}
  color="#000000"
/>

<!-- Particles -->
{#each particles as p}
  <T.Mesh position={p.position}>
    <T.SphereGeometry args={[0.02, 8, 8]} />
    <T.MeshStandardMaterial color={p.color} emissive={p.color} emissiveIntensity={2} />
  </T.Mesh>
{/each}

<!-- Sparks -->
{#each sparks as s}
  <T.Mesh position={s.position} scale={s.life}>
    <T.SphereGeometry args={[0.05, 4, 4]} />
    <T.MeshStandardMaterial 
      color={s.color} 
      emissive={s.color} 
      emissiveIntensity={4} 
      transparent 
      opacity={s.life}
    />
  </T.Mesh>
{/each}
