/**
 * Registers the identity elements (`<elohim-imagodei-device-consent-card>`,
 * `<elohim-imagodei-witness-trail>`, …) from the built `elohim-imagodei`
 * package. Only ever imported dynamically — by the device approval route
 * loader and by {@link IDENTITY_ELEMENTS} when a person submits sign-in or
 * create-account — so no page loads Lit up front and specs never register it.
 */
import 'elohim-imagodei/register';
