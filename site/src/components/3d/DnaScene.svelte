<script>
  import { T, useTask } from '@threlte/core';
  import { OrbitControls, Float } from '@threlte/extras';
  import * as THREE from 'three';

  import { dipralixEvents } from '../../lib/events';

  let nodeCount = 100;
  let nodes = [];
  let connections = [];

  function generateGraph() {
    nodes = Array.from({ length: nodeCount }, (_, i) => ({
      id: i,
      position: [
        (Math.random() - 0.5) * 20,
        (Math.random() - 0.5) * 20,
        (Math.random() - 0.5) * 20
      ],
      size: Math.random() > 0.9 ? 0.8 : Math.random() * 0.3 + 0.1,
      color: Math.random() > 0.9 ? '#ffffff' : ['#6366f1', '#22d3ee', '#f97316', '#a855f7', '#4ade80'][Math.floor(Math.random() * 5)],
      isHotspot: Math.random() > 0.9
    }));

    connections = [];
    for (let i = 0; i < nodeCount; i++) {
      const connectionCount = nodes[i].isHotspot ? 5 : 1;
      for (let j = 0; j < connectionCount; j++) {
        if (Math.random() > 0.5) {
          const target = Math.floor(Math.random() * nodeCount);
          if (target !== i) {
            connections.push([nodes[i].position, nodes[target].position]);
          }
        }
      }
    }
  }

  generateGraph();

  dipralixEvents.subscribe(events => {
    if (events.length === 0) return;
    const lastEvent = events[events.length - 1];
    if (lastEvent.type === 'scan') {
      nodeCount = Math.floor(Math.random() * 150) + 50;
      generateGraph();
      scanY = -10;
    }
  });

  let rotation = 0;
  let scanY = -10;
  useTask((delta) => {
    rotation += delta * 0.1;
    scanY += delta * 5;
    if (scanY > 10) scanY = -10;
  });
</script>

<T.PerspectiveCamera
  makeDefault
  position={[20, 20, 20]}
  fov={35}
>
  <OrbitControls
    enableDamping
    dampingFactor={0.05}
  />
</T.PerspectiveCamera>

<T.AmbientLight intensity={0.5} />
<T.PointLight position={[10, 10, 10]} intensity={1} />

<T.Group rotation={[0, rotation, 0]}>
  {#each nodes as node}
    <T.Mesh position={node.position}>
      <T.SphereGeometry args={[node.size, 16, 16]} />
      <T.MeshStandardMaterial
        color={node.color}
        emissive={node.color}
        emissiveIntensity={node.isHotspot ? 2 : 0.5}
      />
    </T.Mesh>
    {#if node.isHotspot}
      <T.PointLight position={node.position} color={node.color} intensity={0.5} distance={5} />
    {/if}
  {/each}

  {#each connections as [start, end]}
    {@const startVec = new THREE.Vector3(...start)}
    {@const endVec = new THREE.Vector3(...end)}
    {@const distance = startVec.distanceTo(endVec)}
    {@const center = startVec.clone().add(endVec).multiplyScalar(0.5)}
    
    <T.Mesh
      position={[center.x, center.y, center.z]}
      on:create={({ ref }) => ref.lookAt(endVec)}
    >
      <T.CylinderGeometry args={[0.01, 0.01, distance, 8]} />
      <T.MeshStandardMaterial
        color="#ffffff"
        transparent
        opacity={0.05}
      />
    </T.Mesh>
  {/each}

  <!-- Scanning Plane -->
  <T.Mesh position={[0, scanY, 0]} rotation={[-Math.PI / 2, 0, 0]}>
    <T.PlaneGeometry args={[30, 30]} />
    <T.MeshStandardMaterial
      color="#22d3ee"
      transparent
      opacity={0.1}
      side={THREE.DoubleSide}
    />
  </T.Mesh>
</T.Group>

<T.GridHelper args={[50, 50, '#ffffff', '#ffffff']} position={[0, -10, 0]} on:create={({ ref }) => {
  ref.material.transparent = true;
  ref.material.opacity = 0.05;
}} />
