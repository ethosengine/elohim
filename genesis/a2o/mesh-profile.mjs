// The Act I mesh lane's tag selection, shared by cucumber.mjs's `mesh` profile and the
// peer-stage builder (genesis/agentic/compute/stage/build-stage-task.mjs), so a stage's pinned
// scenario inventory is exactly the set the profile runs. It lives outside cucumber.mjs because
// a cucumber config file with a default profile function may export nothing else: any named
// export there is read as a static profile and every run refuses ("Invalid profiles specified").
export const MESH_PROFILE_TAGS = '@e2e and not @wip and not @browser and not @browser-only';
