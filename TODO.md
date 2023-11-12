# TODO

> NB: this isn't supposed to be readable by people other than me *for now*.

- Tasks:
  - [ ] Rendering:
    - [x] Array textures
    - [x] Use GLFW or resolve choppy cursor problem
    - [x] Smaller vertex
    - [ ] Ambient Occlusion
    - [ ] Array texture swapping [1]
    - [ ] Non-block tiles
    - [ ] Entities
  - [ ] Math library:
    - [ ] Add matrix support
    - [ ] Replace glm calls
    - [ ] (Long-term) move to separate crate
  - [ ] Chunk storage:
    - Do chunk generation on a separate thread
    - Block ID limit (16 bit is not enough)
    - Use view-box based indexing [2]
    - Chunk palettes
    - Layers [3]
  - [ ] Gameplay:
    - [ ] Inventory
    - [ ] Entities
      - [ ] Player character
      - [ ] Physics:
        - [ ] Collision
        - [ ] Gravity

- Research/resolve:
  - Rendering:
    - Array textures have a limited layer count. [1]
    - Voxel rendering techniques (for big voxels).
    - Non-block UVs.
    - Entities.
  - Chunk storage:
    - Block ID limit (16 bit is not enough).
    - Runtime:
      - Don't use hash maps, use view-box based indexing? [2]
    - Chunk palettes.
    - Layers (vertical because terrain is horizontal). [3]
