function hasFileExtension(pathname) {
  const lastSegment = pathname.split('/').pop() || '';
  return lastSegment.includes('.');
}

function withSecurityHeaders(response) {
  const headers = new Headers(response.headers);
  headers.set('Content-Security-Policy', [
    "default-src 'self'",
    "base-uri 'none'",
    "frame-ancestors 'none'",
    "object-src 'none'",
    "img-src 'self' data:",
    "font-src 'self' data:",
    "connect-src 'self'",
    "script-src 'self' 'unsafe-inline'",
    "style-src 'self' 'unsafe-inline'",
  ].join('; '));
  headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
  headers.set('X-Content-Type-Options', 'nosniff');
  headers.set('X-Frame-Options', 'DENY');
  headers.set('Permissions-Policy', 'camera=(), microphone=(), geolocation=()');
  headers.set('Cross-Origin-Opener-Policy', 'same-origin');
  headers.set('Cross-Origin-Resource-Policy', 'same-origin');

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname === '/') {
      return withSecurityHeaders(Response.redirect(new URL('/command-center', url), 302));
    }
    if (url.pathname === '/command-center') {
      url.pathname = '/command-center/index.html';
    }

    let response = await env.ASSETS.fetch(new Request(url.toString(), request));

    // SPA-style fallback for extensionless command-center routes.
    if (
      response.status === 404 &&
      request.method === 'GET' &&
      url.pathname.startsWith('/command-center/') &&
      !hasFileExtension(url.pathname)
    ) {
      const fallbackUrl = new URL(request.url);
      fallbackUrl.pathname = '/command-center/index.html';
      response = await env.ASSETS.fetch(new Request(fallbackUrl.toString(), request));
    }

    return withSecurityHeaders(response);
  },
};
