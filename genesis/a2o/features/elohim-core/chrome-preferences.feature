@browser @elohim-core @chrome-preferences
Feature: Chrome preferences follow the person across EPR-app boundaries
  Theme and language controls live in the protocol chrome (omnibar, navigator)
  and persist device-wide through one shared contract, so crossing a bundle
  boundary never resets how the protocol looks or speaks.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip @browser-only
  Scenario: Theme choice persists across the app boundary
    When Matthew navigates to "/lamad" in the browser
    And Matthew clicks the element with testid "nav-theme-inline"
    And Matthew clicks the element with testid "lamad-footer-home-link"
    Then the body data-theme attribute equals the chosen theme
    And there should be no console errors

  @wip @browser-only
  Scenario: Switching to Hebrew flips the chrome to RTL and persists
    When Matthew navigates to "/lamad" in the browser
    And Matthew clicks the element with testid "profile-bubble"
    And Matthew clicks the element with testid "nav-language"
    And Matthew clicks the element with testid "nav-language"
    Then the document dir attribute is "rtl"

  @wip @browser-only @regression
  Scenario: Chrome follows the theme toggle
    # Pins the frozen var-chain class (theme-authority spec C1, alpha
    # 1.0.0-dev-60cc0846): the omnibar and navigator must repaint when the
    # person toggles theme — not just the content below them. Chrome
    # var-chains are declared on :root; the override must reach :root
    # (html[data-theme] authority), or the chrome freezes on the default.
    When Matthew navigates to "/lamad" in the browser
    And Matthew notes the computed background of the navigator chrome
    And Matthew clicks the element with testid "nav-theme-inline"
    Then the computed background of the navigator chrome changes
    And the document root carries the chosen theme in data-theme and color-scheme

  @wip @browser-only @regression
  Scenario: The lamad viewport carries no UA frame
    # Pins the bundle-baseline gap (theme-authority spec D5): a bundle whose
    # styles.scss skips elohim-core/base.scss ships the UA default
    # body{margin:8px} as a visible frame around the whole viewport
    # (alpha 1.0.0-dev-60cc0846, live-probed margin "8px").
    When Matthew navigates to "/lamad" in the browser
    Then the computed margin of the document body is "0px"

  @wip @browser-only @regression
  Scenario: Dark-mode chrome is readable
    # Pins the color-scheme desync + pairing classes (theme-authority spec
    # C2/C3/C4): with the manual dark theme active, every visible chrome
    # text node meets WCAG 1.4.3 — system colors must follow color-scheme,
    # bound surfaces must carry bound foregrounds.
    When Matthew navigates to "/lamad" in the browser
    And Matthew sets the theme to "dark"
    Then every visible text node in the navigator chrome meets contrast "4.5"
    And every visible text node in the omnibar meets contrast "4.5"
