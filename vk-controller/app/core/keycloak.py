from authlib.integrations.starlette_client import OAuth

from app.core.config import settings

oauth = OAuth()
oauth.register(
    name='keycloak',
    client_id=settings.CLIENT_ID,
    client_secret=settings.CLIENT_SECRET,
    server_metadata_url=settings.keycloak_metadata_url,
    client_kwargs={'scope': 'openid profile email offline_access'}
)