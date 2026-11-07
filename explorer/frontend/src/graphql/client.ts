import { ApolloClient, ApolloLink, InMemoryCache, createHttpLink } from '@apollo/client';

const API_KEY = (import.meta.env as { VITE_API_KEY?: string }).VITE_API_KEY;

const httpLink = createHttpLink({
  uri: '/graphql',
});

const authLink = new ApolloLink((operation, forward) => {
  operation.setContext(({ headers = {} }: { headers?: Record<string, string> }) => ({
    headers: {
      ...headers,
      'X-API-Key': API_KEY,
    },
  }));
  return forward(operation);
});

export const client = new ApolloClient({
  link: authLink.concat(httpLink),
  cache: new InMemoryCache(),
  defaultOptions: {
    watchQuery: {
      errorPolicy: 'all',
    },
    query: {
      errorPolicy: 'all',
    },
  },
});
