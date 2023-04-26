import { SharedArray } from 'k6/data';
import http from 'k6/http';

const endpoint = 'http://127.0.0.1:3722';
/**
 * An JSON array containing all of Twitter usernames to be fetched.
 * @type {string[]}
 */
const twitter_ids = new SharedArray('twitter_ids', function() {
  return JSON.parse(open('./twitter_ids.json'));
});

const query = `
query GET_PROFILES_QUERY($platform: String, $identity: String) {
  identity(platform: $platform, identity: $identity) {
    uuid
    platform
    identity
    displayName
    ownedBy {
      uuid
      platform
      identity
      displayName
      __typename
    }
    nft(category: [\"ENS\"]) {
      uuid
      category
      chain
      id
      __typename
    }
    neighborWithTraversal(depth: 5) {
      source
      from {
        uuid
        platform
        identity
        displayName
        ownedBy {
          uuid
          platform
          identity
          displayName
          __typename
        }
        nft(category: [\"ENS\"]) {
          uuid
          category
          chain
          id
          __typename
        }
        __typename
      }
      to {
        uuid
        platform
        identity
        displayName
        ownedBy {
          uuid
          platform
          identity
          displayName
          __typename
        }
        nft(category: [\"ENS\"]) {
          uuid
          category
          chain
          id
          __typename
        }
        __typename
      }
      __typename
    }
    __typename
  }
}
`;

const headers = {
  'Content-Type': 'application/json',
  'Accept': 'application/json',
};

function constructBatch() {
  return twitter_ids.map((id) => {
    return [
      'POST',
      endpoint,
      JSON.stringify({ query, variables: { platform: 'twitter', identity: id }}),
      { headers },
    ]
  })
}

export default function() {
  http.batch(constructBatch())
}
