# CS401 class note

operational expense (OPEX), capital expense (CAPEX) → centralize server

tenant isolation: challenge for cloud provider

cloud provider classification: infrastructure/ platform/ software as a service

data center hierarchy: rack, row & aisle, pod (building unit)

top-of-rack (ToR) switch: 2 per rack, high capacity

- north-south traffic: outside to inside—load balancing
- east-west traffic: within inside—high bandwidth

network topology

- fat tree. problem: require insane link capacity on top
    - link aggregation: bond multiple link together
- leaf-spine topology (clos): layered multiple spine connected to multiple rack
