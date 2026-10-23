@e2e @protocol @concern:content-chrome-trust-edge @act:i
Feature: A person can inspect any content a doorway shows them, from a chrome the content cannot touch
  As a person who opens a piece of Elohim Protocol content in an ordinary web browser
  I want a quiet mark on the page that opens onto what the content is, who holds it, who
  governs it, what it may do on my device and how far it has been witnessed
  So that I can trust what I am looking at for reasons I can inspect, the way a browser's own
  page-info panel lets me inspect a site, without the content being able to change what I am
  told about it, and without any one doorway or web address becoming the thing I am trusting.

  # This feature's concern, content-chrome-trust-edge, names the TRUST EDGE: the place where the
  # protocol meets an ordinary browser, and where a person's trust is either kept inspectable or
  # lost to whoever serves the page.
  #
  # WHY TRUST LIVES IN THE RECORD, NOT THE DOORWAY. On the ordinary web, trust settles on whoever
  # serves the page: the domain, the host, the platform. That is how the web flattens trust and
  # how a few hosts come to capture it. Here every reading the chrome shows is derived from the
  # content's record and from the network that holds and witnesses it, not from the doorway
  # that delivered it. A doorway only delivers and checks. So the same content reaches people
  # through many doorways and many names, and the person sees the same account of it through
  # every one: delivery stays plural, and trust is not captured at the doorway or the address.
  # The delivery itself is not hidden either: which doorway and which peer brought the content is
  # shown as its own reading, so the person can inspect the path without that path becoming the
  # thing they trust.

  # THE RECORD AND THE ADDRESS. Every piece of Elohim Protocol content has an EPR (Elohim
  # Protocol Resource) RECORD: the protocol's account of what the content is, who holds it, who
  # governs it, how far it may be shown and what it needs to run. The record is an account ABOUT
  # the content, not the content's bytes. The record names the content's
  # CONTENT ADDRESS, a fingerprint computed from the content's own bytes, so any bytes can be
  # checked against the record without trusting whoever delivered them.
  #
  # THE CHROME. An ordinary browser was never built for this protocol, so whatever delivers a
  # piece of content (a doorway, or the person's own peer on their own device) sends a small
  # piece of the protocol's own browser along with it: the CHROME. It has three levels, and each
  # costs screen space only when the person asks for it:
  #   1. The CHIP: a small mark at the top of the page, present by default.
  #   2. The OMNIBAR: one thin bar the chip opens.
  #   3. PAGE INFO: a panel opened from one mark on the omnibar, the TRUST MARK. This works like
  #      the icon beside the address in a familiar browser, which opens a panel about the site.
  # Page info is a short list of ROWS. Each row is a one-line reading that opens into a CARD:
  #   - Source: what the content is, its content address, and whether the network has
  #     witnessed its record;
  #   - Delivery: how it reached this person: which doorway answered, under which name, which
  #     peer the bytes came from, and how they were checked against the content address; or,
  #     when no doorway was involved, that it came from the person's own peer;
  #   - Holders: how many households hold it;
  #   - Steward: its STEWARD, the person or community that keeps its record and answers for
  #     it (not the households that hold copies, and not necessarily its author); its REACH,
  #     named as a scope such as the commons (anyone) or one household; and how it is governed;
  #   - Claims: the CLAIMS that back its record, each an assertion recorded about the content
  #     (a review, an inspection, an attestation), separate from the record's own account of
  #     itself and from the network's witness;
  #   - Permissions: what it may do on this device;
  #   - Yours: the person's own relation to it, shown only when they are signed in.
  # Every card ends in a link to the content's EPR PAGE, /epr/<record id>: the full public
  # account of the record, opened on the same doorway and naming the same delivery, so the whole
  # chain by which the content reached the person can be inspected. Nothing the chrome shows is
  # a dead end.
  #
  # WHERE THE CONTENT RUNS. A browser keeps the scripts, storage and sign-in of each web
  # address it visits (each ORIGIN, in browser terms) apart from every other origin's. Every
  # PUBLIC NAME the protocol serves under (a domain name such as alpha.elohim.host) therefore
  # comes as a pair of origins:
  #   - the name itself, for example alpha.elohim.host, serves the chrome, the protocol's own
  #     app and the person's sign-in;
  #   - its APPS ORIGIN, apps. in front of the same name, serves the content's own files.
  # Because the two are different origins, the browser's own same-origin policy stops either one
  # from reading or changing the other: content on apps.alpha.elohim.host cannot touch the page,
  # the chrome or the sign-in on alpha.elohim.host.
  # On a person's own device there is no public name, and the peer keeps the same separation
  # with two of the device's own local addresses (localhost and 127.0.0.1, which a browser also
  # treats as two origins): the chrome on one, the content's files on the other.
  # The pair belongs to the name, not to any one doorway. Every doorway that answers for a name
  # pairs it the same way, whichever peers it fetched the files from.
  # The chrome shows the content in a FRAME: a window within the page whose files come from the
  # apps origin. The content therefore cannot read the person's sign-in, and cannot change what
  # the chrome says about it. This protects the chrome against accidents in every case. It is
  # not a defence against content the protocol has admitted: this network is built to trust its
  # gates.
  #
  # TWO MODES, AND TRUSTFUL WINS. Content earns REACH, how far across the network it may be
  # shown, by passing the protocol's GATES: reviews, inspections and attestations, carried out
  # by agents (people, or software answerable to people) and recorded where anyone can check
  # them. The reach a record carries is what its gates have granted so far. Every piece of
  # Elohim Protocol content a doorway shows has such a record, and is shown only within the
  # reach its record declares; that content is TRUSTFUL. If a reviewer later narrows its reach,
  # people outside the narrower reach are refused it outright, with the reason and where to
  # be heard.
  #   - TRUSTFUL content may use what its record DECLARES it needs (its own storage, forms,
  #     fullscreen, opening windows) without the person being asked, and nothing it does not
  #     declare.
  #   - TRUSTLESS content is anything shown that is not Elohim Protocol content, such as a page
  #     from the wider web shown inside a protocol page. It is run the way an ordinary browser
  #     runs a page it knows nothing about: no storage and nothing extra.
  # Camera, microphone and location need the person's own grant, or their steward's where a
  # stewardship grant (a recorded delegation that lets one party act for another) applies, in
  # both modes; stewardship grants are another story's to prove (see NOT PROVEN HERE). Trustless caution does not limit trustful content: on a conflict, the trustless
  # profile yields to what the trustful record declares.
  #
  # TRUST FOLLOWS DELIVERY. A doorway shows content as soon as its bytes match the record's
  # content address; it does not wait for the network to witness the record. The record is
  # WITNESSED once a peer other than the one that wrote it holds it as valid in the protocol's
  # shared ledger (the DHT, a distributed hash table). The chip shows AMBER until then and GREEN
  # after; while that cannot be confirmed, it stays amber. The doorway does not set the colour
  # itself: it reads the holding peer's answer.
  #
  # WHEN THE CHROME COMES OFF. Only one kind of page may show content without the doorway's
  # chrome: a protocol app that presents content inside its own trust surface and fulfils the
  # same claims. Such an app is a WRAPPER, and its own record declares that it presents
  # content. The protocol's own app is one. It serves each content's RECORD PAGE, which is the
  # same page as its EPR page, /epr/<record id>. The record page shows the content in a frame
  # and, around it, what the content is, who holds it, how it is governed and where it came from. Everywhere else the person gets the chrome:
  #   - opening the content's address;
  #   - opening the raw file's address on the apps origin;
  #   - another website putting the content in a frame of its own.
  #
  # THE TEST VEHICLE. These scenarios run on the HOUSEHOLD: a test network of a few peers on one
  # machine, standing in for a home's devices, with doorway "alpha-A" as its front door. Each
  # scenario publishes a small site this run builds, under a record this run owns. The record
  # declares the site a CONTENT APP (format html5-app): a piece of content packaged as a site.
  # The site is a COHERENT bundle: a small zip whose index page names only files the zip
  # contains, so it can start in a browser. Its ENTRY ADDRESS is the address a person opens it
  # at, /apps/<slug>/<entry file> on a doorway's public name, where the slug names this one
  # site; the RAW ENTRY FILE's address is the same path on the apps origin.
  # Coherence is a precondition here, not a claim:
  # what a doorway does with an incoherent bundle is proven in the app-deliverability story.
  # "This run" is the current test run. "A record this run owns" is a record this run published
  # itself, under a test identity it controls, so it can say what the record declares.
  # The household has three storage peers, each named for the person whose device it
  # stands in for: matthew (behind doorway "alpha-A"), jessica (behind doorway "elohim.host") and
  # james (behind none). The household has ONE public name, elohim.localhost, paired with
  # apps.elohim.localhost; together these are "the household's public names", and any other
  # address is outside them. Both doorways answer for that one name, each on its own port here,
  # as two members of one name would in the field. So the two-doorway scenario proves that
  # two doors of the same name describe content the same way.
  #
  # NOT PROVEN HERE, each named so it stays a decision:
  #   - Amber on demand. A just-published record can reach a second peer within a second here,
  #     and no scenario can hold the network back, so when the first look is already green this
  #     story cannot show amber; the chrome's own tests pin how amber reads.
  #   - Whether the EPR page that every card leads to renders. That is the EPR page's own story.
  #   - Behaviour behind a content delivery network.
  #   - Refusing a person outside a record's reach, with the reason and where to be heard. That
  #     is the doorway's reach story (features/lms/intimate-reach-household.feature); here every
  #     record is shown at commons reach, to anyone within it.
  #   - The desktop app's own isolation. The desktop app (Tauri) does not yet keep content apart
  #     from the person's storage the way a browser keeps two origins apart; that is its own story.
  #   - What the Steward and Claims cards contain. This story proves that each row is there,
  #     that its card leads to the EPR page, and that both doorways show the same reading; the
  #     steward and the claims themselves belong to the records' and the gates' stories.
  #   - Permissions a steward grants on someone's behalf under a stewardship grant, and how the
  #     Permissions card shows them. That is the stewardship story's.
  #   - The trustless mode. No page from the wider web is shown through a doorway today, so no
  #     scenario can open one; the trustless profile is pinned by the chrome's own tests until
  #     content projected inward from the wider web exists to open.

  # THREE KINDS OF PEER, and the steps keep them apart.
  #   - A FEDERATED PEER is a doorway: a front door that federates delivery for a public name.
  #     "alpha-A" and "elohim.host" are the household's two, each in front of a different
  #     storage peer. Each answers for a public name and serves that name's apps origin. In the
  #     steps, federated peer "alpha-A" and doorway "alpha-A" name the same front door.
  #   - A PEER, or storage peer, is a device of the protocol's own peer-to-peer network, where
  #     records are held with authority: matthew, jessica and james on the household.
  #   - A HUB is an always-on peer that serves live aggregates and steadier live channels and
  #     needs no doorway. No scenario here uses one.
  # In "federated peer "X" at "Y"", the value after "at" is the key under which the harness
  # finds that doorway's address.
  # Every scenario carries two tags: @requires:owned-substrate (it needs a household this test run
  # controls, so it is held, not failed, where none is running) and @deliverability-browser (it
  # drives a real browser). The feature's own tags: @act:i places it in Act I, the stories that
  # run on a household this test run controls, and @concern names the habit this story proves.
  Background:
    # Each line registers one doorway, a federated peer. Both answer for the one public name,
    # elohim.localhost; "elohim.host" is the harness's name for the second doorway, not a domain.
    # Behind "alpha-A" stands the storage peer matthew, behind "elohim.host" jessica; james
    # stands behind neither.
    Given federated peer "alpha-A" at "alpha-A"
    And federated peer "elohim.host" at "elohim.host"

  @requires:owned-substrate @deliverability-browser
  Scenario: a person opens content, opens page info from the chip, and every card ends in a link to the content's EPR page
    # "Coherent" and "a record this run owns" are the test vehicle's terms, defined above: a zip
    # whose index page names only files it contains, under a record this run published itself.
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    Then the chip is on the page before the content's frame begins to load
    And the content runs in a frame whose files come from doorway "alpha-A"'s apps origin (apps. in front of its public name)
    When the visitor opens the chip
    Then the omnibar is open across the top of the page, with the trust mark on it
    When the visitor opens the trust mark
    Then page info lists the rows "Source", "Delivery", "Holders", "Steward", "Claims" and "Permissions"
    And page info does not list "Yours", because the visitor is not signed in
    And each row opens a card that ends in a link to this app's EPR page
    When the visitor presses Escape
    Then page info closes and the trust mark has the keyboard focus again

  @requires:owned-substrate @deliverability-browser
  Scenario: the content in the frame cannot reach the chrome or the person's sign-in
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    And the visitor opens page info and reads the holder count in its Holders row
    # The test itself runs this attempt as code inside the content's frame, so the proof does
    # not depend on what the bundle happens to contain.
    And the content in the frame tries to read the page around it and to change the holder count the chrome shows
    Then the browser refuses the content access to the page around it
    And page info's Holders row still shows the holder count the visitor read before
    And the content could not read the sign-in kept by doorway "alpha-A"'s own origin

  # A record this run has just published is witnessed once a second peer holds it, and on a
  # household this small that takes anywhere from under a second to most of a minute. So the
  # first look can be amber or already green, and the scenario holds the chip to the network's
  # own answer either way: green only once a peer other than the author holds the record, and
  # amber turning green, without a reload, once one does. The answer it checks against is the
  # storage peer jessica's: jessica did not write the record, so her peer holds it only once it
  # has reached her. A record the household seeded long ago has been witnessed. The visitor is
  # served immediately in every case.
  @requires:owned-substrate @deliverability-browser
  Scenario: content is shown at once, amber until the network has witnessed its record and green after
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    Then the content was shown without waiting for its record to be witnessed
    And the chip agrees with the network: green only if a peer other than the author already holds the record, and otherwise amber, turning green within 90 seconds once one does
    Given a content app the household seeded when it was set up, whose record the network has since witnessed
    When a visitor opens that seeded app's entry address on federated peer "alpha-A" as a page in a browser
    Then the chip shows green, and page info's Source card says the record is witnessed

  @requires:owned-substrate @deliverability-browser
  Scenario: protocol content may do what its record declares, and is given nothing it does not declare
    Given a coherent EPR app bundle this run just built, which tries to open a new window when it starts
    And an EPR record this run owns for it, declaring that it needs to open windows
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    Then the app inside the frame opened its window
    And page info's Permissions card says the app may open windows, as its record declares
    Given a second coherent EPR app bundle this run just built, which also tries to open a new window when it starts
    And an EPR record this run owns for it, declaring nothing it needs
    And doorway "alpha-A" holds that bundle and knows its record as a content app (format html5-app)
    When a visitor opens that app's entry address on federated peer "alpha-A" as a page in a browser
    Then the app inside the frame was refused its window
    And page info's Permissions card says the app may not open windows, because its record does not declare it

  @requires:owned-substrate @deliverability-browser
  Scenario: the chrome comes off only inside a declared wrapper such as the content's record page, and nowhere else
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    And a page on another website, served from an address outside the household's public names, that frames the raw entry file's address
    # The PROTOCOL'S OWN APP is the app a doorway serves for the protocol's own pages, among them
    # every content's record page. It is a wrapper: its own record declares that it presents
    # content, and the record page shows the content's claims around the content.
    # This step reads the protocol's own app's record and fails unless it declares that it
    # presents content, so the wrapper's standing is checked, not assumed.
    And the protocol's own app on doorway "alpha-A", whose own record declares that it presents content
    # First surface: the raw file opened as a page on the apps origin.
    # The apps origin does not show a content file opened as a page on its own: the doorway
    # redirects the visitor to the content's entry address on its public name, where the
    # chrome loads first and the file runs in the chrome's frame.
    When a visitor opens the raw entry file's address on doorway "alpha-A"'s apps origin as a page
    Then the visitor lands on this app's entry address on doorway "alpha-A", inside the chrome
    # Second surface: another website framing the raw file.
    When a visitor opens that page on the other website
    Then that frame shows the chrome with the app inside it, not the app alone
    # Third surface: the record page, the one place the chrome comes off.
    When a visitor opens this app's record page on doorway "alpha-A"
    Then the app appears inside the record page's own frame, served from the apps origin, with no second chrome around it
    And the record page shows, around the frame, the app's title, who holds it, how it is governed and where it came from

  # Signed in, the person also sees how they stand toward the content. Maria is one of the
  # household's personas whose account lives on doorway "alpha-A".
  @requires:owned-substrate @deliverability-browser
  Scenario: a signed-in person also sees their own relation to the content
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    And the visitor is signed in as household persona "Maria" on doorway "alpha-A"
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    And the visitor opens the chip, then the trust mark on the omnibar it opens
    Then page info lists the rows "Source", "Delivery", "Holders", "Steward", "Claims", "Permissions" and "Yours"
    And the "Yours" card ends in a link to this app's EPR page

  # The network learns from what the chrome sees. When content does something its record did
  # not declare, such as reaching for the page around it, the browser reports it. The doorway
  # judges nothing: it relays the report into the network, where the peers that hold the
  # content record it as a SIGNAL: a small entry, kept with those peers' other feedback about
  # the content, naming the content's address and what the content did. That is the network's own pain
  # signal, and the reviewers of the community that governs the content can act on it by
  # narrowing the content's reach through a gate. This scenario proves the signal arrives;
  # what the reviewers then decide is the gates' story.
  @requires:owned-substrate @deliverability-browser
  Scenario: content that does what its record did not declare raises a signal against its address
    Given a coherent EPR app bundle this run just built, which reaches for the page around it when it starts
    And an EPR record this run owns for it
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    # Thirty seconds covers the browser reporting to the doorway and the doorway passing the
    # report on to the peer; on a household this small both are prompt.
    Then within 30 seconds at least one of the household's storage peers that holds the app has recorded a signal against its content address naming what it reached for, relayed by doorway "alpha-A"

  # Plural delivery, proved: the second doorway sits in front of a different peer, which holds
  # the content's bytes and its record copy on its own. What the person is told about the
  # content does not depend on which door they came through; only the Delivery card differs,
  # because it describes the door.
  @requires:owned-substrate @deliverability-browser
  Scenario: the same content opened through either of the household's doorways is described the same way
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And doorway "alpha-A" holds the bundle and knows its record as a content app (format html5-app)
    And doorway "elohim.host" holds the bundle and knows its record as a content app (format html5-app)
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    And another visitor opens this app's entry address on federated peer "elohim.host" as a page in a browser
    Then both chromes name the same record, the same content address and the same witness state
    And both chromes' Holders and Claims cards read the same
    And each chrome's Delivery card names the doorway the visitor came through and the peer behind it, and the two name different ones
    And each chrome's EPR links lead to the same record, on the doorway the visitor came through, naming that delivery

  # A DOORWAY IS AN OUTER LAYER, NOT A REQUIREMENT. A person's own device runs their own peer,
  # and the chrome must work there with every doorway gone: no web address, no doorway, only
  # their peer and what it holds. The record, its witness, what it may do and the person's own
  # relation to it are all on the person's own peer. What the peer cannot know right now, it
  # says so, with when it last knew. To stand in for "every doorway is gone", the visitor's
  # browser in this scenario is refused every request to any doorway; it can reach only the
  # household storage peer "matthew", as matthew's own device would.
  @requires:owned-substrate @deliverability-browser
  Scenario: on their own device, with every doorway gone, a person still opens content inside the chrome and can inspect it
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And the household storage peer "matthew" holds the bundle and knows its record as a content app (format html5-app)
    And the visitor is matthew, on his own device and signed in as himself, and every doorway is unreachable from it
    # This visit also checks the separation the device keeps without a public name: the chrome
    # loads on one local address and the content's frame on the other.
    When the visitor opens this app's entry address on the household storage peer "matthew" as a page in a browser
    Then the chip is on the page before the content's frame begins to load
    When the visitor opens the chip, then the trust mark on the omnibar it opens
    Then page info lists the rows "Source", "Delivery", "Holders", "Steward", "Claims", "Permissions" and "Yours"
    And page info's Delivery card says the content came from the visitor's own peer and names no doorway
    And page info's Source card states the record's witness as this device knows it
    And page info's Holders card says how many households hold the app as far as this device knows, and when it last knew
    # The record, its steward, its claims and what it declares it needs are on matthew's own peer,
    # so the Steward, Claims and Permissions cards read the same as anywhere else. The last step
    # closes the proof: nothing on this page came through a doorway.
    And no request the browser made reached a doorway
