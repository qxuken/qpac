var hosts = __HOSTS__;
var proxy = __PROXY__;
var DIRECT = "DIRECT;";

var cache = new LRUCache({ capacity: 1000 });

function FindProxyForURL(_url, host) {
  var cachedValue = cache.get(host);
  if (cachedValue) {
    return cachedValue;
  }

  if (binarySearch(host)) {
    cache.put(host, proxy);
    return proxy;
  }

  cache.put(host, DIRECT);
  return DIRECT;
}

function binarySearch(host) {
  var left = 0;
  var right = hosts.length - 1;

  while (left <= right) {
    var mid = (left + right) / 2;

    if (hosts[mid] === host) {
      return true;
    }

    if (host < hosts[mid]) {
      right = mid - 1;
    } else {
      left = mid + 1;
    }
  }

  return false;
}

// https://gist.github.com/lucaong/cc6ac6e65e598217fc6f
function LRUCache(options) {
  this._options = options || {};
  this._map = {};
  this._queue = {};
  this._capacity = this._options.capacity || 10;
  this._size = 0;
}

var _detachFromQueue = function (node, queue) {
  if (node === queue.first) queue.first = node.next;
  if (node === queue.last) queue.last = node.prev;
  if (node.prev != null) node.prev.next = node.next;
  if (node.next != null) node.next.prev = node.prev;
};

var _moveToLast = function (node, queue) {
  node.prev = queue.last;
  node.next = null;
  if (queue.last != null) queue.last.next = node;
  queue.last = node;
  if (queue.first == null) queue.first = node;
};

LRUCache.prototype.put = function (key, value) {
  var replaced = this.delete(key);
  var queue = this._queue;
  var node = { value: value, key: key };
  _moveToLast(node, queue);
  this._map[key] = node;
  this._size += 1;
  if (this._size > this._capacity) this.delete(this._queue.first.key);
  return replaced;
};

LRUCache.prototype.get = function (key) {
  var node = this._map[key];
  if (node == null) return null;
  if (this._options.touchOnGet) {
    _detachFromQueue(node, this._queue);
    _moveToLast(node, this._queue);
  }
  return node.value;
};

LRUCache.prototype.delete = function (key) {
  var node = this._map[key];
  if (node == null) {
    return false;
  } else {
    _detachFromQueue(node, this._queue);
    delete this._map[key];
    this._size -= 1;
    return true;
  }
};

LRUCache.prototype.forEach = function (callback, thisArg) {
  var node = this._queue.first;
  while (node != null) {
    callback.call(thisArg, node.value, node.key);
    node = node.next;
  }
};
