@epic:public_observer
@user_type:board_member
@governance_layer:municipality
@related_users:citizen,developer_interests,politician,activist,community_organizer
@related_layers:district,community,ecological
@elohim_agents:public_observer,domain_elohim,community_elohim,accountability_elohim

Feature: Municipal Board Governance for Board Member
  As a board member serving on a municipal board in the public_observer system
  Operating at the municipality governance layer
  I want independent analysis of land use, housing, and development decisions
  So that I can balance economic development with community interests and equitable outcomes

  Background:
    Given the Elohim Protocol is operational
    And the board_member user is registered in the system
    And the municipality governance context is active
    And I am a volunteer serving on planning commission, housing authority, or municipal board

  Scenario: Planning commissioner evaluates large residential development proposal
    Given developer proposes 500-unit development with affordable housing component
    And staff recommendation supports approval citing housing needs
    And developer presentation emphasizes economic benefits and community amenities
    When the planning commissioner requests Observer independent analysis
    Then they receive comparison of promised versus delivered benefits from developer's past projects
    And see affordable housing calculation showing units actually affordable to local residents
    And access traffic, school, and infrastructure impact analysis
    And discover developer donated $5000 to commissioner's recent campaign
    And independent analysis reveals information staff presentation omitted

  Scenario: Housing authority board member evaluates mixed-income development policy
    Given authority considers policy requiring mixed-income units in new developments
    And staff presents complex financing and legal considerations
    And board member wants to make informed decision but has limited affordable housing expertise
    When the board member uses Observer accessible expertise
    Then they receive plain-language explanation of mixed-income development models
    And see examples from peer housing authorities with outcomes data
    And access key trade-offs between requiring affordability and reducing development
    And get questions to ask staff about implementation and enforcement
    And accessible expertise enables informed policy judgment

  Scenario: Planning commissioner discovers voting pattern aligned with developer interests
    Given commissioner has served for two years making numerous land use decisions
    And local newspaper questions commissioner's independence
    And commissioner wants to understand their own voting record
    When the commissioner uses Observer accountability tracking
    Then they see their votes on developments categorized by community support levels
    And discover they voted for 85% of projects opposed by neighborhood associations
    And find their votes consistently favor developments with higher density than staff recommended
    And access campaign contribution data showing developer funding sources
    And accountability transparency helps commissioner reflect on whose interests they serve

  Scenario: Parks board member tracks implementation of playground improvement promises
    Given board approved contract with playground vendor three years ago
    And vendor promised high-quality equipment meeting accessibility standards
    And community members complain about equipment safety and accessibility
    When the board member uses Observer implementation tracking
    Then they discover equipment doesn't meet accessibility standards promised
    And see maintenance issues documented across multiple park sites
    And find vendor's performance worse than contract specifications
    And access data showing peer municipalities use better vendors
    And impact tracking reveals vendor failed to deliver promised outcomes

  Scenario: Planning commissioner understands community concerns beyond testimony
    Given commission considers rezoning to allow commercial development in residential area
    And supporters include business owners and economic development advocates who testified
    And only 8 residents attended meeting to oppose
    When the commissioner seeks community accountability data from Observer
    Then they see survey data from neighborhood residents showing strong opposition
    And discover working families couldn't attend evening meeting to testify
    And access analysis showing traffic and noise impacts on residential area
    And understand broader community concerns beyond who had time to testify
    And community accountability helps commissioner serve constituent interests

  Scenario: Housing authority board member evaluates rent increase proposal
    Given authority staff recommends rent increases to cover operating costs
    And staff presentation emphasizes authority's fiduciary responsibility and budget needs
    And tenant advocates testify about hardship but lack supporting data
    When the board member requests Observer independent analysis
    Then they receive comparison of authority's costs to peer housing authorities
    And see tenant income data showing rent burden impacts by increase level
    And access analysis of cost drivers and alternative budget approaches
    And discover authority's administrative costs higher than similar authorities
    And independent analysis reveals trade-offs staff presentation minimized

  Scenario: Planning commissioner detects astroturf campaign supporting development
    Given commission faces decision on controversial mall redevelopment
    And receives hundreds of emails and petition signatures supporting project
    And commission wants to understand if support is authentic community sentiment
    When the commissioner uses Observer to analyze advocacy campaign
    Then they discover emails use identical language suggesting coordinated campaign
    And find petition signatures include many from outside the municipality
    And see analysis showing campaign funded and organized by developer
    And access authentic community sentiment data showing more mixed views
    And capture protection helps commissioner distinguish grassroots from astroturf

  Scenario: Municipal board member prepares for complex zoning ordinance revision
    Given planning staff proposes comprehensive zoning code update
    And draft ordinance is 150 pages of technical legal language
    And board must vote within 30 days after public comment period
    When the board member uses Observer decision support
    Then they receive executive summary highlighting key policy changes and impacts
    And see comparison of proposed code to peer municipalities' approaches
    And access analysis of who benefits and who is burdened by major changes
    And get specific questions to ask staff about implementation concerns
    And accessible expertise enables informed judgment on complex ordinance
