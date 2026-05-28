@e2e @storage @wip
Feature: Fair-share limits keep storage honest — no free-riding, no giving it all away
  As the steward of my family's home server
  I want the protocol to hold a fair balance between space I keep, space I lend to friends, and space I share with everyone
  So that I can't accidentally lean on everyone else while giving nothing, and no one can talk me into giving away so much there's nothing left for my own family

  Every home server keeps a fair balance: a slice for the shared commons that
  everyone draws on, a slice for friends and family, a slice for the wider
  groups we belong to, and a slice always kept free for our own use. The
  protocol holds that balance for us, so being generous never quietly turns into
  being taken advantage of.

  Background:
    Given a home server keeps its space in a fair balance of 20% shared commons, 40% friends and family, 25% wider groups, and 15% kept free
    And at least 10% always goes to the shared commons, with no way to opt out

  Scenario: A first promise that fits the fair balance is accepted
    Given Maria's home server has 100 GB of space and no promises made yet
    When Maria promises to keep 30 GB of the Garcia family's content safe
    Then the promise is accepted
    And Maria's storage dashboard shows her balance still within the fair-share limits

  Scenario: A promise that would crowd out her own family is gently refused
    Given Maria has already promised 50 GB to the Garcia family on her 100 GB server
    When Maria tries to promise another 35 GB to a third family
    Then the protocol gently declines, explaining she'd give away more than her fair share to friends and family
    And no promise is recorded
    And the explanation names the fair-share limit she'd cross, not a confusing error

  Scenario: The dashboard tells the truth about promised space versus space actually used
    Given several of Maria's friends want copies of the same shared files
    When Maria opens her home server's dashboard
    Then she sees an honest summary like "Promised: 80 GB; Actually stored: 35 GB"
    And the dashboard explains that one stored copy can keep several promises at once
