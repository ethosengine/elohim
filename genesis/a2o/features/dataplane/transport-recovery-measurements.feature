@e2e @dataplane @local @regression @wip @concern:transport-diversity
Feature: Matthew's household recovery evidence names both resilience and isolation
  Matthew's family should not have to choose a network transport by reputation.
  After the local mesh deliberately takes one household peer down, an operator
  needs one trustworthy timeline that says which transport pairs restored the
  family's writings, which pair was genuinely isolated, and whether the recovery
  destabilised a conductor or restarted without a usable zome path.

  The recovery matrix appends one JSON object per run. This story reads that
  artifact after the matrix finishes; it never churns the mesh itself. Every
  declared scenario must be represented before anyone compares transports.
  A homo pair runs the same transport on both peers; a mixed pair runs a dual
  peer against one single-plane peer; fanout-1 and fanout-2 recover from one
  and two surviving holders respectively.

  The dual warm-return row carries the first useful service threshold: the peer
  must recover in under five minutes. The deliberately split libp2p/iroh pair
  must remain an honest red with the three data-transfer legs P0, P1, and P2
  named, rather than being mistaken for a broken harness. P0 means the sync
  document totals match the survivor, P1 means every blob-backed content row
  has the survivor's blob hash, and P2 means every referenced blob can be read.

  Every row also preserves two diagnostic distinctions. A conductor receipt
  maximum is the largest validation-receipt elapsed time in seconds during the
  recovery window, or null when the log had no in-window sample; null must never
  become a reassuring 0. The restart probe calls a zome path: alive means that
  call worked, dead means it definitively failed, inconclusive means the probe
  answered without deciding, and unknown means no verdict was recorded. These
  states keep HTTP serving from being mistaken for conductor anchoring.

  Scenario: The completed matrix is truthful enough to guide a transport choice
    Then the completed recovery timeline at "$MESH_DIR/recovery-timeline.jsonl" satisfies:
      | rule                            | scenario             | shape | value                           |
      | scenario-present                | homo-libp2p           | any   |                                 |
      | scenario-present                | homo-iroh             | any   |                                 |
      | scenario-present                | homo-dual             | any   |                                 |
      | scenario-present                | mixed-dual-libp2p     | any   |                                 |
      | scenario-present                | mixed-dual-iroh       | any   |                                 |
      | scenario-present                | split-libp2p-iroh     | any   |                                 |
      | scenario-present                | fanout-1              | any   |                                 |
      | scenario-present                | fanout-2              | any   |                                 |
      | recovered-under-seconds         | homo-dual             | warm  | 300                             |
      | expected-red-with-failing-legs | split-libp2p-iroh     | any   | P0,P1,P2                        |
      | every-record-receipt-max        | any                   | any   | number-or-null                  |
      | every-record-zome-path          | any                   | any   | alive,dead,inconclusive,unknown |
