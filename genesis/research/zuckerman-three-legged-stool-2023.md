# The Three-Legged Stool: A Manifesto for a Smaller, Denser Internet

**Authors:** Chand Rajendra-Nicolucci and Michael Sugarman, under editorial direction of Ethan Zuckerman
**Published:** March 29, 2023
**Institution:** Initiative for Digital Public Infrastructure, UMass Amherst
**URL:** https://publicinfrastructure.org/2023/03/29/the-three-legged-stool/
**PDF/ePub:** https://tinyurl.com/idpi3leg

---

## Executive Summary

At the Initiative for Digital Public Infrastructure, we believe that a truly sustainable and resilient digital public sphere is possible and is actively being created. We envision a public sphere supported by these three legs:

- Consists of many different platforms with a wide variety of scales and purposes;
- Users can navigate with a loyal client that aggregates, cross-posts, and curates;
- Is all supported by cross-cutting services rooted in interoperable data.

In this paper, we illustrate our vision for a healthier digital public sphere by exploring what we believe are its three constitutive parts.

First, we propose a *pluriverse* consisting of existing platforms alongside a flourishing ecosystem of Very Small Online Platforms (VSOPs) that serve conversations and communities that are poorly served by today's digital public sphere. Just as we do not exclusively gather in shopping malls in the physical world, Facebook, Twitter, and YouTube are not the right place for every community and conversation online. We argue the need for civic-centered VSOPs like our platform Smalltown. We highlight existing VSOPs like Letterboxd and An Archive of Our Own and discuss what it takes to develop a new VSOP, using our work on Freq, a VSOP dedicated to music discovery and discussion, as a case study.

Second, we sketch out a "loyal client" for navigating the digital public sphere. Akin to an email client like Apple Mail or a chat client like ICQ, our loyal client aggregates, filters, and posts to a person's various social media feeds, be those VSOPs or established platforms like Twitter or Reddit. Such a tool depends on people being able to delegate authority to a loyal client, which comes with challenges related to privacy, adoption, and usability.

Third, we introduce the "friendly neighborhood algorithm store." This is a marketplace that VSOPs and loyal clients can rely on for curation and Trust & Safety, tools which no single VSOP or loyal client could develop on their own, and which large platforms have developed over decades with significant resources. These include recommender systems, spam detection, anti-abuse tools, and powerful filters for CSAM and terrorist content.

We believe this moment, when people are so dissatisfied with the platforms that have dominated for the past decade-and-a-half, presents a unique opportunity to build a digital public sphere where people and communities with different preferences and purposes can participate accordingly.

## Glossary

**Adversarial interoperability** — Connecting two pieces of software without permission from both parties, for the purposes of sharing data between the two.

**Algorithm** — A process automated to accomplish a task or solve a problem. In the context of social media, algorithms are used to organize users' feeds and aid human labor such as content moderation and rule enforcement.

**Loyal client** — A proposed genre of software that helps a user connect to multiple social media feeds while giving that user full control over data shared and how content is organized.

**Digital public infrastructure** — Software, regulations, and norms that support non-corporate and civic spaces online stewarded and governed by the communities who use them.

**Digital public sphere** — In aggregate, the spaces online where all manner of discourse unfolds, be that about politics, culture, civic concerns, or anything else.

**Miyawaki garden** — A restorative ecology technique to reintroduce native biodiversity by planting an extremely high concentration of different species in a small area. Trees in a Miyawaki garden grow about 10 times faster than those grown by the conventions of commercial forestry.

**Pluriverse** — A world containing many worlds, borrowed from the Chiapan Zapatistas by way of the Verses collective.

**VLOP** — Or Very Large Online Platform. As designated by the European Union's Digital Services Act, a platform whose average monthly users account for at least 10% of the EU's population.

**VSOP** — Or Very Small Online Platform. Our model for small platforms serving specific purposes for specific communities, which will populate the pluriverse and ensure a robust, resilient digital public sphere.

## 1. Introduction

2022 was a shockingly chaotic year for large-scale, corporate social media.

The company formerly known as Facebook moved fast and broke things (notably its market valuation) pivoting from the boring and messy world of conventional social media to a daring virtual reality world where nobody has legs: the Metaverse. The nearly 3 billion people on Facebook and Instagram be damned, the 200,000 monthly active users on Meta's Horizon Worlds VR platform are the company's priority now.

Likewise, Twitter's new owner Elon Musk has transformed the company from one that ships features a bit slowly—a cadence perhaps inspired by former CEO Jack Dorsey's frequent yoga retreats—to one in rapid decline.

In the years prior to 2022, we realized just how important the digital public sphere had become. In 2022, we realized just how risky it is leaving the digital public sphere in the care of capricious billionaires.

Many of our colleagues make persuasive cases for using regulation to create a better Facebook and a better Twitter. Others make compelling arguments for why we need to leave these mega-platforms in the dust, decamping for decentralized alternatives in the Fediverse.

We think the path to a healthier digital public sphere includes both of those solutions. But we also think neither alone, nor both in conjunction, are the full answer. At the Initiative for Digital Public Infrastructure, we believe that a truly sustainable and resilient digital public sphere consists of many different platforms with a wide variety of sizes and purposes, that users can navigate with a loyal client that aggregates, cross-posts, and curates, all supported by cross-cutting services rooted in interoperable data.

In this paper, we sketch the outline of an entire ecosystem, and a way to begin implementing this vision through small steps. In part, we are inspired by an example from ecology: the Miyawaki method. A small movement is underway in ecological restoration to bring biodiversity back to environs either devastated by urban development or monoculture farming. Reintroducing native biodiversity is advantageous for the natural world and our planet's climate for multiple reasons: it lends habitat to local species whose populations are dwindling, it reinvigorates the soil, and it sequesters atmospheric carbon. The issue is that reforestation takes a really long time.

Enter the Miyawaki forest. Restorative ecologists work in increments of hectares and plant a wide variety of native seedlings in tight quarters, about 20- to 30-times more densely than they would typically be planted in commercial forestry. Trees in this habitat grow about 10 times faster than in typical reforestation. What is the recipe to the success of the Miyawaki forest? Density and diversity, on a small scale.

Now, we need much more than a hectare of diverse forest to restore entire ecosystems. But that tract is a start: it's both a proof of concept and a scalable model. That modest hectare gives a new home to species that struggled to gain a foothold in previously hostile environs, ones stamped out by blight and monoculture. And, as those Miyawaki forests develop, they grow outwards too, spreading that nurtured biodiversity.

We are ultimately hoping to accomplish something similar with the digital public sphere. We are advocating for growing small, dense, and diverse spaces to improve the online world... and we need your help to do it.

## 2. Gardening in the Pluriverse

The European Union's Digital Services Act (DSA) went into effect in November 2022. It is the most comprehensive effort in the West to regulate the Internet to date. The DSA trains its crosshairs on a powerful band of corporate actors, what the legislation calls Very Large Online Platforms and Very Large Online Search Engines, or VLOPs and VLOSEs.

Regulating VLOPs and VLOSEs is important. The EU rightly recognizes that much of the digital public sphere is shaped by these players, and reshaping the digital public sphere demands confronting them head-on.

However, we need more than just regulation. We need a fertile, flexible ecosystem of Very Small Online Platforms which serve different purposes than the existing VLOPs. This is a vision of a pluriverse: a "world where many worlds may fit."

Unlike Facebook's all-encompassing designs for its metaverse, our vision of the pluriverse acknowledges that some of these worlds would be VLOPs, and many more would be the independent small platforms that we describe.

The result would be a more diverse ecosystem online and, in turn, a more sustainable and resilient digital public sphere. The pluriverse is conceived to foster a life online that allows for the cultivation of independent spaces with different goals, norms, governance, and affordances than what's available on VLOPs.

At iDPI, we tend to think of existing social media platforms as if they were rooms. If Facebook were a room, it would be a massive convention center. Facebook is a good place to randomly run into people you know and drop in on various groups, maybe even to see a few keynotes from people who have managed to gain access to the loudspeaker.

But a big convention center is not a great place to have an Alcoholics Anonymous meeting. It is not conducive to holding a debate about changing the speed limit on Main Street. You probably would not attend an after-work yoga class in the convention center, if you had other options. Offline we have spaces like church basements, town halls, and yoga studios. Why should it be any different online?

We do not believe the convention center can ever host a good town hall meeting. Both its affordances (the technical capabilities that govern what people can do in an online space) and its norms (the ways people have learned to use the space) inhibit constructive, civil conversations. We need new spaces purpose-built for the conversations and communities that are poorly served, ignored, or marginalized by VLOPs. They are not convention centers. Our VSOPs are neither rooms for all people nor for all purposes.

Much like with the Miyawaki forest, we see these spaces as smaller tracts that take time and care to cultivate. VSOPs should encourage a kind of stewardship, wherein users feel compelled to uphold norms and participate in governance to achieve the specific goals of these platforms. Once this succeeds on a small scale, those healthy communities can spread.

VSOPs are not speculative fiction. They exist and, in some cases, are quite popular. Letterboxd is a platform built for exactly one thing: discussing movies. Users can create a watchlist, submit ratings, write reviews, and aggregate movies into thematic lists. Other users comment on reviews and on those lists. Influencers emerge, often because they are notable film critics or comedians.

There is no room to talk about anything unrelated to movies on the platform. It is not somewhere you can have a conversation about makeup, Cajun recipes, or even television shows. You cannot stream a movie natively. There is no way to form a group. There is just your feed, thousands of collections and reviews, and some commenting functionality.

Letterboxd is run by a team of nine full-time employees and has never relied on any venture capital funding. It generates sustainable revenue through ad sales and premium subscriptions. It has no ambitions to be all things to all people and, as a result, is very good at what it does.

### Civic Social Media

There is one genre of VSOP we are especially invested in: spaces that host civic conversations at the local level. Nextdoor and Facebook are useful for finding people's pets and sourcing recommendations for contractors. They are ill-equipped to host conversations about controversial local issues like the middle school curriculum or the proposed zoning amendment—indeed, controversial conversations have a history of turning ugly in lightly moderated local spaces. We need spaces for discussing local issues which are closer to the experience of a newspaper's editorial page than a shouting match at a local bar.

We are experimenting with one model, a platform called Smalltown. Smalltown hosts local civic discussions. It is closely moderated and the discussion is scaffolded. Moderators are active guides and participants in the conversation, posting prompts, following up on comments and questions, and generally encouraging constructive participation. Smalltown instances are the opposite of "build it and they will come." They require a great deal of active work to cultivate.

Smalltown initially focused on discourse around local civic meetings like town halls, enabling a community to participate in an asynchronous, persistent digital space before, during, and after meetings. Users could access the conversation whenever and wherever they had internet access. This was a conscious choice based on feedback from local government officials who worried that physical town meetings excluded many community members: parents, people who work nights, people for whom English is not a first language. Beyond just opening up public meetings, Smalltown has now expanded its focus to all kinds of local civic discussions.

Smalltown is not the only project of this ilk. Our friends at Front Porch Forum operate a social media platform that mainly services towns in Vermont. Posts go live once a day after being reviewed by a professional moderator and the platform is supported through advertising by local businesses. It functions as something of a healthier blend of Nextdoor and Craigslist.

In the Netherlands, our friends at Public Spaces are experimenting with civic social media, operated in conjunction with public broadcasters and academics. One project we are excited about is PubHubs, a tool for creating private digital social hubs for your family, your classroom, your basketball team, and so on. Each space is established and moderated to meet the specific needs of a particular group and may have different norms and rules than other groups hosted on the same architecture.

### Sprouting a new VSOP

What does it take to sprout a new VSOP? It starts with a community or conversation that is poorly served by existing platforms. And just like with transforming a vacant lot into a community garden, it takes a lot of work by a dedicated group of people.

One example that we take inspiration from is An Archive of Our Own, a volunteer-powered platform launched in 2009 by fanfiction authors: people who write original stories using characters and worlds from published books, television shows, and movies.

Those authors have long had difficulty maintaining their work on existing social media platforms. Overzealous copyright enforcement has led to fanfic authors being deplatformed from several spaces. Fed up with platforms that didn't understand their work or their culture, a group of (mostly women) authors designed and built their own platform specifically to meet the needs of their community.

AO3, as it is colloquially known, serves millions of people a month, and includes tools specific to the needs of fans. As our colleague Casey Fiesler has chronicled, this has fostered design cues such as a bespoke architecture for trigger and content warnings, the ability to divorce an author's name from a piece of writing in the archive, and an emphasis on accessibility. It also includes a careful "tagging" system that allows viewers to find content that meets their interests and avoid subjects they know they want to steer clear of.

In the process of building their own VSOP, the founders of AO3 also created a nonprofit advocacy organization and a scholarly journal. Their online community building has led to powerful offline community building as well.

There are considerable hurdles to starting a new VSOP. In particular, it's difficult to customize, control, and run the software needed to host a small social network. Existing commercial solutions are expensive and limit a community's ability to control and customize the software and data. Open-source software often requires technical expertise to set up, manage, and customize.

This means that the subset of people who are able to experiment with small social networks is limited to those able to pay for technical experimentation and those with the knowledge to experiment themselves. To support a flowering of small social networks, we need a system that enables people with minimal technical expertise and money to spin up their own custom and controllable social networks. It should be as straightforward for someone to create and customize a social network as it is for them to start a blog on WordPress or start a webstore with Shopify.

Smalltown takes a step towards that goal by managing the software for our partners and offering a number of customizations that don't require people to code. For example, we enable communities to turn on a "post queue", which makes it so all posts submitted to the site have to be approved by a moderator before appearing on the site, with the click of a button. Similarly, communities can add and remove direct messaging with the click of a button.

A VSOP also needs a governance system. We think that this should involve a healthy dose of community governance in most cases. One reason is that many VSOPs will likely receive most of their revenue from the communities they serve, similar to how a local newspaper or nonprofit is funded. This suggests some form of community participation in governance to ensure accountability. Community governance also lends legitimacy, allowing a platform to make decisions that some members disagree with, without causing those people to distrust the platform or exit it entirely. Further, community governance can help train people for healthier civic participation more broadly, by teaching them good habits for holding productive meetings, resolving disagreements, articulating their point of view, and seeking common ground.

A VSOP also needs revenues, though fewer than you probably think. Hosting a small social network usually costs less than $500 a year in technical costs. Providing moderation and scaffolding is time consuming, and requires either dedicated volunteers or additional revenues. There are many different models that can work, including subscriptions, advertising, donations, licensing, foundation support, and public funding. The appropriate model(s) will depend on the VSOP.

Succeeding on these fronts hinges on buy-in from an engaged community. One way to encourage that is to start a VSOP to fill a specific hole in the social media landscape. We are attempting to do exactly that with a project we are calling Freq, a platform for social music discovery. Freq takes inspiration from Letterboxd and the social networking aspects of private torrent communities. It lets users organize into subreddit-like groups to talk music and create publicly visible thematic collections of albums. Groups can rely on a variety of tools for customization, moderation, and governance.

Freq's initial development is an experiment in online community formation and governance without a revenue model, but it will likely transition to a mix of revenue from advertising and premium subscriptions. Institutional licensing may be an option too, as there is potential for partnership with public libraries and college and non-profit radio stations.

We are exploring Smalltown and Freq because they provide spaces that do not exist elsewhere in the pluriverse. We are committed to building tools that make it easier to create these new spaces, and to partnering with people—especially people in marginalized communities—to share lessons about their creation and cultivation.

## 3. A Loyal Client

The second part of our vision is the "loyal client"—a tool that aggregates, organizes, and posts to the various platforms people are a part of. This is an old idea which seeks to solve a common problem on the internet: what you want to do as a user, and what a web server may want to do, often come into conflict.

Web browsers block pop-ups and ad trackers because the server wants a user's attention and data, and the user would rather not hand them over. Email clients block spam, allow you to customize your inbox, and give you new tools like schedulers and auto-complete because users want control over how they interact with email servers.

The idea of a loyal client for social media has antecedents in Usenet news clients and universal chat clients like ICQ, but fell by the wayside as Web 2.0 behemoths such as Facebook, Twitter, and YouTube grew to dominate our digital public sphere. The loyal client suffered a near fatal blow as usage of the internet moved primarily to the mobile phone: clients created by the big tech companies became vastly more popular than accessing web servers through the web browser, which continues to serve as a loyal client on the desktop.

We think a loyal client is critical to moving past many of the hard problems and sticky debates of the digital public sphere. By moving some power from platforms into the hands of individuals and clients, we can split the gordian knot of conflicting visions for social media, giving users more control over where and how they participate, and what they see. We believe many platforms, combined with a loyal client, will allow many different answers to the difficult questions of the digital public sphere, and inject some much needed agency and innovation into the social media space.

At iDPI we are building our own loyal client, Gobo. The first version of Gobo was created in 2017 to explore the idea of choosing your own settings for how to view social media. The tool was a demonstration of the idea of increased user control, but was quite slow and lacked some important features, like cross-posting, which limited its adoption.

We have revived the project for a 2023 release, in part as a response to the exodus from Twitter to Mastodon, which has demonstrated the need for a tool which allows you to read and write to both platforms from a single interface. The app we plan to launch in Spring 2023 allows you to read and post content on Twitter, Mastodon, and Reddit. In the future, we hope to include platforms like Facebook, Instagram, and LinkedIn as well.

Once we have an initial version supporting aggregation and cross-posting, we will roll out a new architecture for choosing, customizing, and testing "lenses" for your feed. We want to give people control over the algorithms that sort their feeds—this could be for a feed from a single platform, or for a multi-platform aggregated feed. That means that instead of having to use Twitter's algorithm to construct your feed, you could choose one that reflects your preferences better, for example, creating a feed with less political content, more pictures of dogs, and nothing with the word "Musk" in it. Lenses can filter out content you want to avoid, can introduce new content you may be interested in, and can rank content according to your preferences.

We want to support a third-party ecosystem of algorithm providers, which would provide more variety and innovation than we could ever create on our own, by designing and propagating an open standard for developing and integrating third-party algorithms. It's important that users be able to audit these algorithms, inspecting the decisions an algorithm made when constructing their feed. Gobo users will also be able to arbitrarily test posts against an algorithm to see how it performs and change the algorithm's settings. And while existing clients for Facebook and other services vacuum up huge swaths of user data, we are minimizing the data collected about user activity, avoiding unnecessary tracking and always asking for consent for the data we must collect.

We think a loyal client is essential for a healthy, robust, and pluralistic digital public sphere. Smaller platforms are always going to have fewer posts and less traffic. We need a tool that reminds you of those conversations and enables you to switch in and out of participating in different spaces, large and small, seamlessly. Again, we are not calling to abolish VLOPs—it is unrealistic to ask people to give up their networks on Facebook and Twitter, and we recognize the value that those networks can provide—we just think people should be able to post where they want and read what they want.

This is a challenging project that includes significant privacy, adoption, usability, and legal challenges.

### Privacy

A loyal client needs to be able to fetch all the content that a person is subscribed to across different platforms. However, many platforms, particularly some of the most popular platforms like Facebook and Instagram, do not allow third party clients to fetch and display content.

Platforms often cite privacy concerns when explaining why they block third party clients, conflating commercial data scraping operations with third-party clients that their users choose to use. We agree that data scraping can be harmful, but arguing that users choosing a third-party client poses a privacy threat akin to commercial data scraping is absurd.

The key difference is consent: the user decides whether to grant the third-party client access to their feed—the client is not scraping their feed and displaying it without their consent. Some may argue that the people who appear in the user's feed did not consent to their content being sent to a third-party client. We believe this situation is addressed by the idea of contextual privacy: by allowing someone to subscribe to your content, you implicitly trust them to handle your content with care.

### Adoption

Regardless of our arguments about privacy, some platforms will still choose to not volunteer interoperability. To address this, we can turn to adversarial interoperability and regulation.

Adversarial interoperability is "when you create a new product or service that plugs into the existing ones without the permission of the companies that make them." If a platform chooses to not make a well documented API available, a loyal client could still interoperate with them, taking more adversarial approaches such as reverse engineering their API or downloading content and storing it locally.

There's a good historical example of successful adversarial interoperability. In the early days of instant messaging, you could only chat with other people who used the same proprietary chat client. So if you used MSN, you couldn't chat with someone who used Yahoo!. Israeli company Mirabilis created ICQ, a third-party client that figured out how to connect these proprietary clients, which quickly became a user favorite.

Regulations that support interoperability are another way to address platforms that refuse to interoperate voluntarily. The EU recently passed a regulation, the Digital Markets Act, that mandates large platforms make their messaging interoperable. Other legislation could similarly require that social media platforms make their content available to third-party clients, or offer protections for adversarial interoperability.

### Usability

Building a loyal client raises some serious usability challenges. How do you maintain important context when aggregating content from many different platforms? How do you make a universal client as seamless and easy to understand as using a major platform directly? How do you support discovery? Can you give people more control while avoiding choice paralysis?

We don't have the answers to all of these questions, but we do have some starting points.

First, powerful defaults will be important to avoid choice paralysis and manage complexity. People need to find the tool useful almost immediately, without the need for a lot of set-up.

Second, we believe algorithms are useful and necessary for aggregation—reverse chronological order is not a panacea. A compelling universal client experience would allow you to do things like create filters for local content, sports content, friends content, and political content across platforms while also providing a more general "For You" algorithm.

Third, new design patterns for displaying, filtering, and organizing content will need to be created and tested, and there are significant engineering problems related to scale, speed, and algorithmic customization that need to be tackled to enable intuitive and powerful designs.

### Legal Challenges

We expect that at least some VLOPs will act to block Gobo and may take legal action against us. We hope to use any legal challenges as an opportunity to talk about the value of loyal clients, their history in computing, and their benefits to users. We flag the possibility of legal risk because threats of lawsuits too often stop legitimate research and tools designed to favor user interests over corporate interests.

## 4. The Friendly Neighborhood Algorithm Store

Similar to how WordPress has a robust plugin marketplace for building out your blog, to support our vision of a pluriverse navigated with a loyal client, we will need a "friendly neighborhood algorithm store", that serves both VSOPs and loyal clients.

Large platforms have developed bespoke systems that support curation and Trust & Safety over decades. VSOPs typically lack the resources and experience necessary to develop comprehensive tools on their own. Without a marketplace for algorithms that can help them quickly implement a robust Trust & Safety architecture, VSOPs risk putting their users in harm's way and facing litigation.

Similarly, in order for a loyal client or a VSOP to serve an individual's preferences, they need to be able to select from a wide variety of algorithms that reflect those preferences. It's unlikely that a loyal client or VSOP would be able to create such a collection itself—a third party marketplace seems necessary.

### What types of algorithms do we think are needed?

A far from comprehensive list:

- Spam detection
- CSAM detection and reporting
- Mitigating abuse and harassment
- Detecting mis- and disinformation
- Personalized "For You" recommendations or network highlights
- Categorization tools that feature or filter content by topics such as sports, local, comedy, news, politics, and so on

Local legal environments may also require:

- Copyright fingerprinting and DMCA takedown
- Identification of violent extremist content

Some of these tools already exist, though they are often hard to implement or overfitted to specific platforms. For example, Facebook has developed a tool, in partnership with the Center for Missing and Exploited Children, to identify Child Sexual Abuse Material (CSAM) by checking digital fingerprints of images against a database of fingerprints of known CSAM. While this database is a powerful tool in the battle against CSAM, it is not accessible to the administrator of a Mastodon server who wishes to scan her content to prevent dissemination of CSAM. We will need resources available to small social network platforms.

It's important that there be a quality assurance process for the friendly neighborhood algorithm store. Poorly crafted algorithms can do more harm than good. A good baseline for what we should be looking for is algorithms that are tuneable, auditable, combinable, and understandable.

- **Tuneable**: you should have meaningful settings that allow you to use the algorithm in different ways.
- **Auditable**: you should be able to throw arbitrary content at the algorithm and see how it responds. You should be able to investigate the data underlying the algorithm and the decisions it made.
- **Combinable**: you should be able to use more than one algorithm and have them work together. Algorithms should be able to work across different source platforms.
- **Understandable**: you should have a sense of what the algorithm is trying to do and how you can control it.

The friendly neighborhood algorithm store is key to restoring choice and innovation to the digital public sphere. Without a marketplace for flexible and powerful algorithms that support curation and Trust & Safety, VSOPs and loyal clients will struggle to provide quality experiences. The overhead is simply too much for any one VSOP or loyal client to develop the algorithms they need on their own.

To attract players to this marketplace, there needs to be the promise of a sustainable business model. One idea is revenue sharing: dominant platforms pay a commission to third party filters based on their popularity. However, this would require dominant platforms to offer their users a menu of third party algorithms, something that may not happen without regulation. A loyal client could use the same model. Another possibility is third party algorithm providers could charge users directly for using their service. Finally, public funding could support some third party algorithm providers—governments and foundations could agree to fund prosocial algorithms.

We see real promise in charging users directly. Akismet has built a multi-million dollar, 80-plus-person company solving the very real problem of blog spam with a plug-in for WordPress.

## Conclusion

As promised, the future for social media we outline is significantly more complicated than pithy advice to move to a more open platform or to quit social media entirely. However, we believe the vision we're outlining here is no more complicated than it needs to be.

VSOPs allow different affordances, norms, and rules to meet the needs of different communities. In the process, we open the intriguing possibility that helping to manage a community social network may become a form of civic education in the way Robert Putnam speculated joining the Elks Lodge or managing a local bowling league once was.

The loyal client helps us choose where and how we participate in the digital public sphere, answering difficult questions with different answers for different people. And both loyal clients and VSOPs will need a diverse menu of algorithms to help manage their experiences.

We do not expect to build this ecosystem ourselves: indeed, the idea of the pluriverse is that this ecosystem will be built through thousands of experiments. We are committed to building open source systems as proofs of concept, hoping that our successes and failures will inform other experiments in the space. We also pledge to back away from our experiments as others find better ways to solve the problems we seek to address.

In particular, we believe that organizations committed to the public sphere, including libraries, newspapers, public broadcasters, cultural institutions, local governments and others, have a role to play in reenvisioning the digital public sphere. We believe that spaces to discuss our collective past, present, and future are public goods and should be funded as such. The costs of operating individual VSOPs are low, but the costs of developing loyal clients, tools to support VSOPs, and algorithms that serve the public will be substantial. We believe the architecture we are outlining here is a digital public infrastructure and that at least some of the funding required to realize this vision should come from public coffers.

Finally, we acknowledge that we alone cannot be the ones to solve the problems of the digital public sphere. The people most affected by the shortcomings of contemporary social media are members of marginalized communities. As we experiment, we commit to partnering with a broad range of communities, working to alter tools and practices based on lessons learned.

We believe that social media can be a tool for profound social benefit and transformation. We hope this vision sparks a reaction, whether it's enthusiastic support and willingness to collaborate, or equally enthusiastic desire to advocate for other models. What is most important to us is that we do not continue to accept our current sclerotic, monopolistic approach to the digital public sphere.

## Acknowledgements

We acknowledge the helpful input—and pushback—from friends and colleagues including Eli Pariser, Deepti Doshi, Tom Tyler, Tracey Meares, Bart Jacobs, Jose Van Dijck, Geert-Jan Bogaerts, Nathan Matias, Nathan Schneider, Brian Levine, Tim Berners-Lee, Alex Fink, Cory Doctorow, Daphne Keller, Bill Thompson, Bruno Patino, Dominique Cardon, Vincent Hendricks and many others.

This document was primarily authored by Chand Rajendra-Nicolucci and Michael Sugarman under the editorial direction of Ethan Zuckerman, with additional editing by Rebecca Curran with contributions from the entire iDPI team. iDPI branding was developed by Dario Stefanutto. This document was designed by Michael Sugarman.

Our work at the Initiative for Digital Public Infrastructure has been supported by the John D. and Catherine T. MacArthur foundation and by The Ford Foundation. We are grateful to support from the Knight First Amendment Institute at Columbia University which incubated this work, and to the University of Massachusetts Amherst for ongoing support of our research, publishing and teaching on these topics.
