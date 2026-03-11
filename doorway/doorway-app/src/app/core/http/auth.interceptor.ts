/**
 * Auth HTTP Interceptor
 *
 * Attaches the stored JWT token to all outgoing HTTP requests
 * so doorway-app can authenticate with the doorway backend.
 */

import { HttpInterceptorFn } from '@angular/common/http';

const AUTH_TOKEN_KEY = 'doorway_auth_token';

/**
 * Functional HTTP interceptor that attaches Bearer token from localStorage.
 */
export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const token = localStorage.getItem(AUTH_TOKEN_KEY);

  if (token) {
    const authReq = req.clone({
      setHeaders: { Authorization: `Bearer ${token}` },
    });
    return next(authReq);
  }

  return next(req);
};
