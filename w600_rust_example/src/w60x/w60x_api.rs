#![allow(non_camel_case_types)]
#![allow(dead_code)]

use cty::*;

extern "C" {
    pub fn wm_printf(format: *const c_char, ...) -> c_void;
}
pub const WM_SUCCESS: i32 = 0;
pub const WM_FAILED: i32 = -1;
pub const TLS_PARAM_STATUS_OK: i32 = 0;
pub const TLS_PARAM_STATUS_EINVALID: i32 = 1;
pub const TLS_PARAM_STATUS_EMEM: i32 = 2;
pub const TLS_PARAM_STATUS_EIO: i32 = 3;
pub const TLS_PARAM_STATUS_EPERM: i32 = 4;
pub const TLS_PARAM_STATUS_EINVALIDID: i32 = 5;
pub const TRUE: i32 = 1;
// pub const FALSE:i32 = 0;
// pub type BOOL = i32;

/** cpu clock: 80Mhz */
// pub const CPU_CLK_80M:u32 = 0;
/** cpu clock: 40Mhz */
pub const CPU_CLK_40M: u32 = 1;
// pub const CPU_CLK_16M:u32 = 2;
pub const HZ: u32 = 500;
pub type tls_os_task_t = *mut c_void;
pub type tls_os_queue_t = c_void;
pub type OS_STK = u32;
pub type tls_timer_irq_callback = extern "C" fn(arg: *mut c_void);

/** Wi-Fi states */
#[repr(C)]
#[derive(Debug)]
pub enum tls_wifi_states {
    WM_WIFI_DISCONNECTED,
    /**< Disconnected state */
    WM_WIFI_SCANNING,
    /**< Scanning for a network */
    WM_WIFI_JOINING,
    /**< Trying to join with a BSS/SSID */
    WM_WIFI_JOINED, /*< All authentication completed */
}

/** timer unit */
#[repr(C)]
#[derive(Debug)]
pub enum tls_timer_unit {
    TLS_TIMER_UNIT_US, // < microsecond level(us)
    TLS_TIMER_UNIT_MS, // < millisecond level(ms)
}
pub const WM_TIMER_ID_INVALID: u8 = 0xFF;

/** timer configuration */
#[repr(C)]
pub struct tls_timer_cfg {
    pub unit: tls_timer_unit,             // < timer accuracy
    pub timeout: u32,                     // < timeout period
    pub is_repeat: bool,                  // < cycle timer
    pub callback: tls_timer_irq_callback, // < timeout callback function
    pub arg: *mut c_void,                 // < parameter fot the timeout callback function
}

/** ENUMERATION definition of OS STATUS */
#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub enum tls_os_status {
    TLS_OS_SUCCESS = 0,
    TLS_OS_ERROR,
    TLS_OS_ERR_TIMEOUT,
}

// pub type tls_os_status_t = tls_os_status;
// pub type tls_os_task_t = *mut c_void;
pub type tls_os_status_t = tls_os_status;
pub const TLS_FLS_STATUS_OK: i32 = 0;
pub const TLS_FLS_STATUS_EINVAL: i32 = 1;
// pub const TLS_FLS_STATUS_EBUSY:i32 = 2;pub const TLS_FLS_STATUS_EPERM: i32 = 3;
// pub const TLS_FLS_STATUS_ENOSUPPORT:i32 = 4;
// pub const TLS_FLS_STATUS_EEXIST:i32 = 5;
// pub const TLS_FLS_STATUS_ENOMEM:i32 = 6;
// pub const TLS_FLS_STATUS_EOVERFLOW:i32 = 7;
// pub const TLS_FLS_STATUS_ENODEV:i32 = 8;
// pub const TLS_FLS_STATUS_EDEV:i32 = 9;pub const TLS_FLS_STATUS_EIO: i32 = 10;pub const TLS_FLS_STATUS_ENODRV: i32 = 11;
pub const TLS_PARAM_ID_WPROTOCOL: i32 = 10;
pub const TLS_PARAM_IEEE80211_INFRA: i32 = 1;
pub const TLS_PARAM_ID_IP: i32 = 3;

/** MACRO for callback EVENT to join AP or create soft-AP successfully  */
pub const NETIF_WIFI_JOIN_SUCCESS: u8 = 0x1;
/** MACRO for callback EVENT to fail to join AP */
pub const NETIF_WIFI_JOIN_FAILED: u8 = 0x2;
/** MACRO for callback EVENT to disconnect from AP or destroy soft-AP */
pub const NETIF_WIFI_DISCONNECTED: u8 = 0x3;
/** MACRO for callbck EVENT to get IP address */
pub const NETIF_IP_NET_UP: u8 = 0x4;

/**   Structure of IP parameter    */
#[repr(C)]
pub struct tls_param_ip {
    pub dhcp_enable: u8,
    pub res: [u8; 3],
    pub ip: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns1: [u8; 4],
    pub dns2: [u8; 4],
}

impl Default for tls_param_ip {
    fn default() -> Self {
        tls_param_ip {
            dhcp_enable: 0,
            res: [0u8; 3],
            ip: [0u8; 4],
            netmask: [0u8; 4],
            gateway: [0u8; 4],
            dns1: [0u8; 4],
            dns2: [0u8; 4],
        }
    }
}

/** This is the aligned version of ip4_addr_t,
used as local variable, on the stack, etc. */
#[repr(C)]
pub struct ip4_addr {
    pub addr: u32,
}

/** ip4_addr_t uses a struct for convenience only, so that the same defines can
 * operate both on ip4_addr_t as well as on ip4_addr_p_t. */
pub type ip4_addr_t = ip4_addr;
pub type ip_addr_t = ip4_addr_t;

#[repr(C)]
pub struct tls_ethif {
    pub ip_addr: ip_addr_t,
    pub netmask: ip_addr_t,
    pub gw: ip_addr_t,
    // #if TLS_CONFIG_IPV6
    //     ip_addr_t ip6_addr[IPV6_ADDR_MAX_NUM];
    // #endif
    pub dns1: ip_addr_t,
    pub dns2: ip_addr_t,
    pub status: u8, //0:net down; 1:net up
                    // #if TLS_CONFIG_IPV6
                    //     u8 ipv6_status[IPV6_ADDR_MAX_NUM];      //0:net down; 1:net up
                    // #endif
}

/*
 * NOTE: These public enums are part of the abi.  If you want to add one,
 * add it at where specified so existing users are unaffected.
 */
/** enum lws_callback_reasons - reason you're getting a protocol callback */
#[repr(C)]
#[derive(Debug)]
pub enum lws_callback_reasons {
    LWS_CALLBACK_ESTABLISHED = 0,
    /**< (VH) after the server completes a handshake with an incoming
     * client.  If you built the library with ssl support, in is a
     * pointer to the ssl struct associated with the connection or NULL.*/
    LWS_CALLBACK_CLIENT_CONNECTION_ERROR = 1,
    /**< the request client connection has been unable to complete a
     * handshake with the remote server.  If in is non-NULL, you can
     * find an error string of length len where it points to
     *
     * Diagnostic strings that may be returned include
     *
     *     	"getaddrinfo (ipv6) failed"
     *     	"unknown address family"
     *     	"getaddrinfo (ipv4) failed"
     *     	"set socket opts failed"
     *     	"insert wsi failed"
     *     	"lws_ssl_client_connect1 failed"
     *     	"lws_ssl_client_connect2 failed"
     *     	"Peer hung up"
     *     	"read failed"
     *     	"HS: URI missing"
     *     	"HS: Redirect code but no Location"
     *     	"HS: URI did not parse"
     *     	"HS: Redirect failed"
     *     	"HS: Server did not return 200"
     *     	"HS: OOM"
     *     	"HS: disallowed by client filter"
     *     	"HS: disallowed at ESTABLISHED"
     *     	"HS: ACCEPT missing"
     *     	"HS: ws upgrade response not 101"
     *     	"HS: UPGRADE missing"
     *     	"HS: Upgrade to something other than websocket"
     *     	"HS: CONNECTION missing"
     *     	"HS: UPGRADE malformed"
     *     	"HS: PROTOCOL malformed"
     *     	"HS: Cannot match protocol"
     *     	"HS: EXT: list too big"
     *     	"HS: EXT: failed setting defaults"
     *     	"HS: EXT: failed parsing defaults"
     *     	"HS: EXT: failed parsing options"
     *     	"HS: EXT: Rejects server options"
     *     	"HS: EXT: unknown ext"
     *     	"HS: Accept hash wrong"
     *     	"HS: Rejected by filter cb"
     *     	"HS: OOM"
     *     	"HS: SO_SNDBUF failed"
     *     	"HS: Rejected at CLIENT_ESTABLISHED"
     */
    LWS_CALLBACK_CLIENT_FILTER_PRE_ESTABLISH = 2,
    /**< this is the last chance for the client user code to examine the
     * http headers and decide to reject the connection.  If the
     * content in the headers is interesting to the
     * client (url, etc) it needs to copy it out at
     * this point since it will be destroyed before
     * the CLIENT_ESTABLISHED call */
    LWS_CALLBACK_CLIENT_ESTABLISHED = 3,
    /**< after your client connection completed
     * a handshake with the remote server */
    LWS_CALLBACK_CLOSED = 4,
    /**< when the websocket session ends */
    LWS_CALLBACK_CLOSED_HTTP = 5,
    /**< when a HTTP (non-websocket) session ends */
    LWS_CALLBACK_RECEIVE = 6,
    /**< data has appeared for this server endpoint from a
     * remote client, it can be found at *in and is
     * len bytes long */
    LWS_CALLBACK_RECEIVE_PONG = 7,
    /**< servers receive PONG packets with this callback reason */
    LWS_CALLBACK_CLIENT_RECEIVE = 8,
    /**< data has appeared from the server for the client connection, it
     * can be found at *in and is len bytes long */
    LWS_CALLBACK_CLIENT_RECEIVE_PONG = 9,
    /**< clients receive PONG packets with this callback reason */
    LWS_CALLBACK_CLIENT_WRITEABLE = 10,
    /**<  If you call lws_callback_on_writable() on a connection, you will
     * get one of these callbacks coming when the connection socket
     * is able to accept another write packet without blocking.
     * If it already was able to take another packet without blocking,
     * you'll get this callback at the next call to the service loop
     * function.  Notice that CLIENTs get LWS_CALLBACK_CLIENT_WRITEABLE
     * and servers get LWS_CALLBACK_SERVER_WRITEABLE. */
    LWS_CALLBACK_SERVER_WRITEABLE = 11,
    /**< See LWS_CALLBACK_CLIENT_WRITEABLE */
    LWS_CALLBACK_HTTP = 12,
    /**< an http request has come from a client that is not
     * asking to upgrade the connection to a websocket
     * one.  This is a chance to serve http content,
     * for example, to send a script to the client
     * which will then open the websockets connection.
     * in points to the URI path requested and
     * lws_serve_http_file() makes it very
     * simple to send back a file to the client.
     * Normally after sending the file you are done
     * with the http connection, since the rest of the
     * activity will come by websockets from the script
     * that was delivered by http, so you will want to
     * return 1; to close and free up the connection. */
    LWS_CALLBACK_HTTP_BODY = 13,
    /**< the next len bytes data from the http
     * request body HTTP connection is now available in in. */
    LWS_CALLBACK_HTTP_BODY_COMPLETION = 14,
    /**< the expected amount of http request body has been delivered */
    LWS_CALLBACK_HTTP_FILE_COMPLETION = 15,
    /**< a file requested to be sent down http link has completed. */
    LWS_CALLBACK_HTTP_WRITEABLE = 16,
    /**< you can write more down the http protocol link now. */
    LWS_CALLBACK_FILTER_NETWORK_CONNECTION = 17,
    /**< called when a client connects to
     * the server at network level; the connection is accepted but then
     * passed to this callback to decide whether to hang up immediately
     * or not, based on the client IP.  in contains the connection
     * socket's descriptor. Since the client connection information is
     * not available yet, wsi still pointing to the main server socket.
     * Return non-zero to terminate the connection before sending or
     * receiving anything. Because this happens immediately after the
     * network connection from the client, there's no websocket protocol
     * selected yet so this callback is issued only to protocol 0. */
    LWS_CALLBACK_FILTER_HTTP_CONNECTION = 18,
    /**< called when the request has
     * been received and parsed from the client, but the response is
     * not sent yet.  Return non-zero to disallow the connection.
     * user is a pointer to the connection user space allocation,
     * in is the URI, eg, "/"
     * In your handler you can use the public APIs
     * lws_hdr_total_length() / lws_hdr_copy() to access all of the
     * headers using the header enums lws_token_indexes from
     * libwebsockets.h to check for and read the supported header
     * presence and content before deciding to allow the http
     * connection to proceed or to kill the connection. */
    LWS_CALLBACK_SERVER_NEW_CLIENT_INSTANTIATED = 19,
    /**< A new client just had
     * been connected, accepted, and instantiated into the pool. This
     * callback allows setting any relevant property to it. Because this
     * happens immediately after the instantiation of a new client,
     * there's no websocket protocol selected yet so this callback is
     * issued only to protocol 0. Only wsi is defined, pointing to the
     * new client, and the return value is ignored. */
    LWS_CALLBACK_FILTER_PROTOCOL_CONNECTION = 20,
    /**< called when the handshake has
     * been received and parsed from the client, but the response is
     * not sent yet.  Return non-zero to disallow the connection.
     * user is a pointer to the connection user space allocation,
     * in is the requested protocol name
     * In your handler you can use the public APIs
     * lws_hdr_total_length() / lws_hdr_copy() to access all of the
     * headers using the header enums lws_token_indexes from
     * libwebsockets.h to check for and read the supported header
     * presence and content before deciding to allow the handshake
     * to proceed or to kill the connection. */
    LWS_CALLBACK_OPENSSL_LOAD_EXTRA_CLIENT_VERIFY_CERTS = 21,
    /**< if configured for
     * including OpenSSL support, this callback allows your user code
     * to perform extra SSL_CTX_load_verify_locations() or similar
     * calls to direct OpenSSL where to find certificates the client
     * can use to confirm the remote server identity.  user is the
     * OpenSSL SSL_CTX* */
    LWS_CALLBACK_OPENSSL_LOAD_EXTRA_SERVER_VERIFY_CERTS = 22,
    /**< if configured for
     * including OpenSSL support, this callback allows your user code
     * to load extra certifcates into the server which allow it to
     * verify the validity of certificates returned by clients.  user
     * is the server's OpenSSL SSL_CTX* */
    LWS_CALLBACK_OPENSSL_PERFORM_CLIENT_CERT_VERIFICATION = 23,
    /**< if the libwebsockets vhost was created with the option
     * LWS_SERVER_OPTION_REQUIRE_VALID_OPENSSL_CLIENT_CERT, then this
     * callback is generated during OpenSSL verification of the cert
     * sent from the client.  It is sent to protocol[0] callback as
     * no protocol has been negotiated on the connection yet.
     * Notice that the libwebsockets context and wsi are both NULL
     * during this callback.  See
     *  http://www.openssl.org/docs/ssl/SSL_CTX_set_verify.html
     * to understand more detail about the OpenSSL callback that
     * generates this libwebsockets callback and the meanings of the
     * arguments passed.  In this callback, user is the x509_ctx,
     * in is the ssl pointer and len is preverify_ok
     * Notice that this callback maintains libwebsocket return
     * conventions, return 0 to mean the cert is OK or 1 to fail it.
     * This also means that if you don't handle this callback then
     * the default callback action of returning 0 allows the client
     * certificates. */
    LWS_CALLBACK_CLIENT_APPEND_HANDSHAKE_HEADER = 24,
    /**< this callback happens
     * when a client handshake is being compiled.  user is NULL,
     * in is a char **, it's pointing to a char * which holds the
     * next location in the header buffer where you can add
     * headers, and len is the remaining space in the header buffer,
     * which is typically some hundreds of bytes.  So, to add a canned
     * cookie, your handler code might look similar to:
     *
     *	char **p = (char **)in;
     *
     *	if (len < 100)
     *		return 1;
     *
     *	*p += sprintf(*p, "Cookie: a=b\x0d\x0a");
     *
     *	return 0;
     *
     * Notice if you add anything, you just have to take care about
     * the CRLF on the line you added.  Obviously this callback is
     * optional, if you don't handle it everything is fine.
     *
     * Notice the callback is coming to protocols[0] all the time,
     * because there is no specific protocol negotiated yet. */
    LWS_CALLBACK_CONFIRM_EXTENSION_OKAY = 25,
    /**< When the server handshake code
     * sees that it does support a requested extension, before
     * accepting the extension by additing to the list sent back to
     * the client it gives this callback just to check that it's okay
     * to use that extension.  It calls back to the requested protocol
     * and with in being the extension name, len is 0 and user is
     * valid.  Note though at this time the ESTABLISHED callback hasn't
     * happened yet so if you initialize user content there, user
     * content during this callback might not be useful for anything.
     * Notice this callback comes to protocols[0]. */
    LWS_CALLBACK_CLIENT_CONFIRM_EXTENSION_SUPPORTED = 26,
    /**< When a client
     * connection is being prepared to start a handshake to a server,
     * each supported extension is checked with protocols[0] callback
     * with this reason, giving the user code a chance to suppress the
     * claim to support that extension by returning non-zero.  If
     * unhandled, by default 0 will be returned and the extension
     * support included in the header to the server.  Notice this
     * callback comes to protocols[0]. */
    LWS_CALLBACK_PROTOCOL_INIT = 27,
    /**< One-time call per protocol, per-vhost using it, so it can
     * do initial setup / allocations etc */
    LWS_CALLBACK_PROTOCOL_DESTROY = 28,
    /**< One-time call per protocol, per-vhost using it, indicating
     * this protocol won't get used at all after this callback, the
     * vhost is getting destroyed.  Take the opportunity to
     * deallocate everything that was allocated by the protocol. */
    LWS_CALLBACK_WSI_CREATE = 29,
    /**< outermost (earliest) wsi create notification to protocols[0] */
    LWS_CALLBACK_WSI_DESTROY = 30,
    /**< outermost (latest) wsi destroy notification to protocols[0] */
    LWS_CALLBACK_GET_THREAD_ID = 31,
    /**< lws can accept callback when writable requests from other
     * threads, if you implement this callback and return an opaque
     * current thread ID integer. */
    /* external poll() management support */
    LWS_CALLBACK_ADD_POLL_FD = 32,
    /**< lws normally deals with its poll() or other event loop
     * internally, but in the case you are integrating with another
     * server you will need to have lws sockets share a
     * polling array with the other server.  This and the other
     * POLL_FD related callbacks let you put your specialized
     * poll array interface code in the callback for protocol 0, the
     * first protocol you support, usually the HTTP protocol in the
     * serving case.
     * This callback happens when a socket needs to be
     * added to the polling loop: in points to a struct
     * lws_pollargs; the fd member of the struct is the file
     * descriptor, and events contains the active events
     *
     * If you are using the internal lws polling / event loop
     * you can just ignore these callbacks. */
    LWS_CALLBACK_DEL_POLL_FD = 33,
    /**< This callback happens when a socket descriptor
     * needs to be removed from an external polling array.  in is
     * again the struct lws_pollargs containing the fd member
     * to be removed.  If you are using the internal polling
     * loop, you can just ignore it. */
    LWS_CALLBACK_CHANGE_MODE_POLL_FD = 34,
    /**< This callback happens when lws wants to modify the events for
     * a connection.
     * in is the struct lws_pollargs with the fd to change.
     * The new event mask is in events member and the old mask is in
     * the prev_events member.
     * If you are using the internal polling loop, you can just ignore
     * it. */
    LWS_CALLBACK_LOCK_POLL = 35,
    /**< These allow the external poll changes driven
     * by lws to participate in an external thread locking
     * scheme around the changes, so the whole thing is threadsafe.
     * These are called around three activities in the library,
     *	- inserting a new wsi in the wsi / fd table (len=1)
     *	- deleting a wsi from the wsi / fd table (len=1)
     *	- changing a wsi's POLLIN/OUT state (len=0)
     * Locking and unlocking external synchronization objects when
     * len == 1 allows external threads to be synchronized against
     * wsi lifecycle changes if it acquires the same lock for the
     * duration of wsi dereference from the other thread context. */
    LWS_CALLBACK_UNLOCK_POLL = 36,
    /**< See LWS_CALLBACK_LOCK_POLL, ignore if using lws internal poll */
    LWS_CALLBACK_OPENSSL_CONTEXT_REQUIRES_PRIVATE_KEY = 37,
    /**< if configured for including OpenSSL support but no private key
     * file has been specified (ssl_private_key_filepath is NULL), this is
     * called to allow the user to set the private key directly via
     * libopenssl and perform further operations if required; this might be
     * useful in situations where the private key is not directly accessible
     * by the OS, for example if it is stored on a smartcard.
     * user is the server's OpenSSL SSL_CTX* */
    LWS_CALLBACK_WS_PEER_INITIATED_CLOSE = 38,
    /**< The peer has sent an unsolicited Close WS packet.  in and
     * len are the optional close code (first 2 bytes, network
     * order) and the optional additional information which is not
     * defined in the standard, and may be a string or non-human- readable data.
     * If you return 0 lws will echo the close and then close the
     * connection.  If you return nonzero lws will just close the
     * connection. */
    LWS_CALLBACK_WS_EXT_DEFAULTS = 39,
    /**<  */
    LWS_CALLBACK_CGI = 40,
    /**<  */
    LWS_CALLBACK_CGI_TERMINATED = 41,
    /**<  */
    LWS_CALLBACK_CGI_STDIN_DATA = 42,
    /**<  */
    LWS_CALLBACK_CGI_STDIN_COMPLETED = 43,
    /**<  */
    LWS_CALLBACK_ESTABLISHED_CLIENT_HTTP = 44,
    /**<  */
    LWS_CALLBACK_CLOSED_CLIENT_HTTP = 45,
    /**<  */
    LWS_CALLBACK_RECEIVE_CLIENT_HTTP = 46,
    /**<  */
    LWS_CALLBACK_COMPLETED_CLIENT_HTTP = 47,
    /**<  */
    LWS_CALLBACK_RECEIVE_CLIENT_HTTP_READ = 48,
    /**<  */
    LWS_CALLBACK_HTTP_BIND_PROTOCOL = 49,
    /**<  */
    LWS_CALLBACK_HTTP_DROP_PROTOCOL = 50,
    /**<  */
    LWS_CALLBACK_CHECK_ACCESS_RIGHTS = 51,
    /**<  */
    LWS_CALLBACK_PROCESS_HTML = 52,
    /**<  */
    LWS_CALLBACK_ADD_HEADERS = 53,
    /**<  */
    LWS_CALLBACK_SESSION_INFO = 54,
    /**<  */
    LWS_CALLBACK_GS_EVENT = 55,
    /**<  */
    LWS_CALLBACK_HTTP_PMO = 56,
    /**< per-mount options for this connection, called before
     * the normal LWS_CALLBACK_HTTP when the mount has per-mount
     * options
     */
    LWS_CALLBACK_CLIENT_HTTP_WRITEABLE = 57,
    /**< when doing an HTTP type client connection, you can call
     * lws_client_http_body_pending(wsi, 1) from
     * LWS_CALLBACK_CLIENT_APPEND_HANDSHAKE_HEADER to get these callbacks
     * sending the HTTP headers.
     *
     * From this callback, when you have sent everything, you should let
     * lws know by calling lws_client_http_body_pending(wsi, 0)
     */
    /****** add new things just above ---^ ******/
    LWS_CALLBACK_USER = 1000,
    /* <  user code can use any including / above without fear of clashes */
}

/*
 * NOTE: These public enums are part of the abi.  If you want to add one,
 * add it at where specified so existing users are unaffected.
 */
#[repr(C)]
#[derive(Debug)]
pub enum lws_extension_callback_reasons {
    LWS_EXT_CB_SERVER_CONTEXT_CONSTRUCT = 0,
    LWS_EXT_CB_CLIENT_CONTEXT_CONSTRUCT = 1,
    LWS_EXT_CB_SERVER_CONTEXT_DESTRUCT = 2,
    LWS_EXT_CB_CLIENT_CONTEXT_DESTRUCT = 3,
    LWS_EXT_CB_CONSTRUCT = 4,
    LWS_EXT_CB_CLIENT_CONSTRUCT = 5,
    LWS_EXT_CB_CHECK_OK_TO_REALLY_CLOSE = 6,
    LWS_EXT_CB_CHECK_OK_TO_PROPOSE_EXTENSION = 7,
    LWS_EXT_CB_DESTROY = 8,
    LWS_EXT_CB_DESTROY_ANY_WSI_CLOSING = 9,
    LWS_EXT_CB_ANY_WSI_ESTABLISHED = 10,
    LWS_EXT_CB_PACKET_RX_PREPARSE = 11,
    LWS_EXT_CB_PACKET_TX_PRESEND = 12,
    LWS_EXT_CB_PACKET_TX_DO_SEND = 13,
    LWS_EXT_CB_HANDSHAKE_REPLY_TX = 14,
    LWS_EXT_CB_FLUSH_PENDING_TX = 15,
    LWS_EXT_CB_EXTENDED_PAYLOAD_RX = 16,
    LWS_EXT_CB_CAN_PROXY_CLIENT_CONNECTION = 17,
    LWS_EXT_CB_1HZ = 18,
    LWS_EXT_CB_REQUEST_ON_WRITEABLE = 19,
    LWS_EXT_CB_IS_WRITEABLE = 20,
    LWS_EXT_CB_PAYLOAD_TX = 21,
    LWS_EXT_CB_PAYLOAD_RX = 22,
    LWS_EXT_CB_OPTION_DEFAULT = 23,
    LWS_EXT_CB_OPTION_SET = 24,
    LWS_EXT_CB_OPTION_CONFIRM = 25,
    LWS_EXT_CB_NAMED_OPTION_SET = 26,
    /****** add new things just above ---^ ******/
}

/* enum lws_token_indexes
 * these have to be kept in sync with lextable.h / minilex.c
 *
 * NOTE: These public enums are part of the abi.  If you want to add one,
 * add it at where specified so existing users are unaffected.
 */
#[repr(C)]
#[derive(Debug)]
pub enum lws_token_indexes {
    WSI_TOKEN_GET_URI = 0,
    WSI_TOKEN_POST_URI = 1,
    WSI_TOKEN_OPTIONS_URI = 2,
    WSI_TOKEN_HOST = 3,
    WSI_TOKEN_CONNECTION = 4,
    WSI_TOKEN_UPGRADE = 5,
    WSI_TOKEN_ORIGIN = 6,
    WSI_TOKEN_DRAFT = 7,
    WSI_TOKEN_CHALLENGE = 8,
    WSI_TOKEN_EXTENSIONS = 9,
    WSI_TOKEN_KEY1 = 10,
    WSI_TOKEN_KEY2 = 11,
    WSI_TOKEN_PROTOCOL = 12,
    WSI_TOKEN_ACCEPT = 13,
    WSI_TOKEN_NONCE = 14,
    WSI_TOKEN_HTTP = 15,
    WSI_TOKEN_HTTP2_SETTINGS = 16,
    WSI_TOKEN_HTTP_ACCEPT = 17,
    WSI_TOKEN_HTTP_AC_REQUEST_HEADERS = 18,
    WSI_TOKEN_HTTP_IF_MODIFIED_SINCE = 19,
    WSI_TOKEN_HTTP_IF_NONE_MATCH = 20,
    WSI_TOKEN_HTTP_ACCEPT_ENCODING = 21,
    WSI_TOKEN_HTTP_ACCEPT_LANGUAGE = 22,
    WSI_TOKEN_HTTP_PRAGMA = 23,
    WSI_TOKEN_HTTP_CACHE_CONTROL = 24,
    WSI_TOKEN_HTTP_AUTHORIZATION = 25,
    WSI_TOKEN_HTTP_COOKIE = 26,
    WSI_TOKEN_HTTP_CONTENT_LENGTH = 27,
    WSI_TOKEN_HTTP_CONTENT_TYPE = 28,
    WSI_TOKEN_HTTP_DATE = 29,
    WSI_TOKEN_HTTP_RANGE = 30,
    WSI_TOKEN_HTTP_REFERER = 31,
    WSI_TOKEN_KEY = 32,
    WSI_TOKEN_VERSION = 33,
    WSI_TOKEN_SWORIGIN = 34,

    WSI_TOKEN_HTTP_COLON_AUTHORITY = 35,
    WSI_TOKEN_HTTP_COLON_METHOD = 36,
    WSI_TOKEN_HTTP_COLON_PATH = 37,
    WSI_TOKEN_HTTP_COLON_SCHEME = 38,
    WSI_TOKEN_HTTP_COLON_STATUS = 39,

    WSI_TOKEN_HTTP_ACCEPT_CHARSET = 40,
    WSI_TOKEN_HTTP_ACCEPT_RANGES = 41,
    WSI_TOKEN_HTTP_ACCESS_CONTROL_ALLOW_ORIGIN = 42,
    WSI_TOKEN_HTTP_AGE = 43,
    WSI_TOKEN_HTTP_ALLOW = 44,
    WSI_TOKEN_HTTP_CONTENT_DISPOSITION = 45,
    WSI_TOKEN_HTTP_CONTENT_ENCODING = 46,
    WSI_TOKEN_HTTP_CONTENT_LANGUAGE = 47,
    WSI_TOKEN_HTTP_CONTENT_LOCATION = 48,
    WSI_TOKEN_HTTP_CONTENT_RANGE = 49,
    WSI_TOKEN_HTTP_ETAG = 50,
    WSI_TOKEN_HTTP_EXPECT = 51,
    WSI_TOKEN_HTTP_EXPIRES = 52,
    WSI_TOKEN_HTTP_FROM = 53,
    WSI_TOKEN_HTTP_IF_MATCH = 54,
    WSI_TOKEN_HTTP_IF_RANGE = 55,
    WSI_TOKEN_HTTP_IF_UNMODIFIED_SINCE = 56,
    WSI_TOKEN_HTTP_LAST_MODIFIED = 57,
    WSI_TOKEN_HTTP_LINK = 58,
    WSI_TOKEN_HTTP_LOCATION = 59,
    WSI_TOKEN_HTTP_MAX_FORWARDS = 60,
    WSI_TOKEN_HTTP_PROXY_AUTHENTICATE = 61,
    WSI_TOKEN_HTTP_PROXY_AUTHORIZATION = 62,
    WSI_TOKEN_HTTP_REFRESH = 63,
    WSI_TOKEN_HTTP_RETRY_AFTER = 64,
    WSI_TOKEN_HTTP_SERVER = 65,
    WSI_TOKEN_HTTP_SET_COOKIE = 66,
    WSI_TOKEN_HTTP_STRICT_TRANSPORT_SECURITY = 67,
    WSI_TOKEN_HTTP_TRANSFER_ENCODING = 68,
    WSI_TOKEN_HTTP_USER_AGENT = 69,
    WSI_TOKEN_HTTP_VARY = 70,
    WSI_TOKEN_HTTP_VIA = 71,
    WSI_TOKEN_HTTP_WWW_AUTHENTICATE = 72,

    WSI_TOKEN_PATCH_URI = 73,
    WSI_TOKEN_PUT_URI = 74,
    WSI_TOKEN_DELETE_URI = 75,

    WSI_TOKEN_HTTP_URI_ARGS = 76,
    WSI_TOKEN_PROXY = 77,
    WSI_TOKEN_HTTP_X_REAL_IP = 78,
    WSI_TOKEN_HTTP1_0 = 79,

    /****** add new things just above ---^ ******/
    /* use token storage to stash these internally, not for
     * user use */
    _WSI_TOKEN_CLIENT_SENT_PROTOCOLS,
    _WSI_TOKEN_CLIENT_PEER_ADDRESS,
    _WSI_TOKEN_CLIENT_URI,
    _WSI_TOKEN_CLIENT_HOST,
    _WSI_TOKEN_CLIENT_ORIGIN,
    _WSI_TOKEN_CLIENT_METHOD,

    /* always last real token index*/
    WSI_TOKEN_COUNT,

    /* parser state additions, no storage associated */
    WSI_TOKEN_NAME_PART,
    WSI_TOKEN_SKIPPING,
    WSI_TOKEN_SKIPPING_SAW_CR,
    WSI_PARSING_COMPLETE,
    WSI_INIT_TOKEN_MUXURL,
}

/**
 * typedef lws_callback_function() - User server actions
 * \param wsi:	Opaque websocket instance pointer
 * \param reason:	The reason for the call
 * \param user:	Pointer to per-session user data allocated by library
 * \param in:		Pointer used for some callback reasons
 * \param len:	Length set for some callback reasons
 *
 *	This callback is the way the user controls what is served.  All the
 *	protocol detail is hidden and handled by the library.
 *
 *	For each connection / session there is user data allocated that is
 *	pointed to by "user".  You set the size of this user data area when
 *	the library is initialized with lws_create_server.
 */
pub type lws_callback_function = extern "C" fn(
    wsi: *mut c_void,
    reason: lws_callback_reasons,
    user: *mut c_void,
    _in: *mut c_void,
    len: size_t,
) -> c_int;

/**
 * typedef lws_extension_callback_function() - Hooks to allow extensions to operate
 * \param context:	Websockets context
 * \param ext:	This extension
 * \param wsi:	Opaque websocket instance pointer
 * \param reason:	The reason for the call
 * \param user:	Pointer to ptr to per-session user data allocated by library
 * \param in:		Pointer used for some callback reasons
 * \param len:	Length set for some callback reasons
 *
 *	Each extension that is active on a particular connection receives
 *	callbacks during the connection lifetime to allow the extension to
 *	operate on websocket data and manage itself.
 *
 *	Libwebsockets takes care of allocating and freeing "user" memory for
 *	each active extension on each connection.  That is what is pointed to
 *	by the user parameter.
 *
 *	LWS_EXT_CB_CONSTRUCT:  called when the server has decided to
 *		select this extension from the list provided by the client,
 *		just before the server will send back the handshake accepting
 *		the connection with this extension active.  This gives the
 *		extension a chance to initialize its connection context found
 *		in user.
 *
 *	LWS_EXT_CB_CLIENT_CONSTRUCT: same as LWS_EXT_CB_CONSTRUCT
 *		but called when client is instantiating this extension.  Some
 *		extensions will work the same on client and server side and then
 *		you can just merge handlers for both CONSTRUCTS.
 *
 *	LWS_EXT_CB_DESTROY:  called when the connection the extension was
 *		being used on is about to be closed and deallocated.  It's the
 *		last chance for the extension to deallocate anything it has
 *		allocated in the user data (pointed to by user) before the
 *		user data is deleted.  This same callback is used whether you
 *		are in client or server instantiation context.
 *
 *	LWS_EXT_CB_PACKET_RX_PREPARSE: when this extension was active on
 *		a connection, and a packet of data arrived at the connection,
 *		it is passed to this callback to give the extension a chance to
 *		change the data, eg, decompress it.  user is pointing to the
 *		extension's private connection context data, in is pointing
 *		to an lws_tokens struct, it consists of a char * pointer called
 *		token, and an int called token_len.  At entry, these are
 *		set to point to the received buffer and set to the content
 *		length.  If the extension will grow the content, it should use
 *		a new buffer allocated in its private user context data and
 *		set the pointed-to lws_tokens members to point to its buffer.
 *
 *	LWS_EXT_CB_PACKET_TX_PRESEND: this works the same way as
 *		LWS_EXT_CB_PACKET_RX_PREPARSE above, except it gives the
 *		extension a chance to change websocket data just before it will
 *		be sent out.  Using the same lws_token pointer scheme in in,
 *		the extension can change the buffer and the length to be
 *		transmitted how it likes.  Again if it wants to grow the
 *		buffer safely, it should copy the data into its own buffer and
 *		set the lws_tokens token pointer to it.
 *
 *	LWS_EXT_CB_ARGS_VALIDATE:
 */
type lws_extension_callback_function = extern "C" fn(
    context: *mut c_void,
    ext: *const lws_extension,
    wsi: *mut c_void,
    reason: lws_extension_callback_reasons,
    user: *mut c_void,
    _in: *mut c_void,
    len: size_t,
) -> c_int;

/** struct lws_protocols -	List of protocols and handlers client or server
 *					supports. */

#[repr(C)]
pub struct lws_protocols {
    pub name: *const c_char,
    /**< Protocol name that must match the one given in the client
     * Javascript new WebSocket(url, 'protocol') name. */
    pub callback: Option<lws_callback_function>,
    /**< The service callback used for this protocol.  It allows the
     * service action for an entire protocol to be encapsulated in
     * the protocol-specific callback */
    pub per_session_data_size: size_t,
    /**< Each new connection using this protocol gets
     * this much memory allocated on connection establishment and
     * freed on connection takedown.  A pointer to this per-connection
     * allocation is passed into the callback in the 'user' parameter */
    pub rx_buffer_size: size_t,
    /**< lws allocates this much space for rx data and informs callback
     * when something came.  Due to rx flow control, the callback may not
     * be able to consume it all without having to return to the event
     * loop.  That is supported in lws.
     *
     * This also controls how much may be sent at once at the moment,
     * although this is likely to change.
     */
    pub id: c_uint,
    /**< ignored by lws, but useful to contain user information bound
     * to the selected protocol.  For example if this protocol was
     * called "myprotocol-v2", you might set id to 2, and the user
     * code that acts differently according to the version can do so by
     * switch (wsi->protocol->id), user code might use some bits as
     * capability flags based on selected protocol version, etc. */
    pub user: *mut c_void, /*< ignored by lws, but user code can pass a pointer
                                   here it can later access from the protocol callback */

                           /* Add new things just above here ---^
                            * This is part of the ABI, don't needlessly break compatibility */
}

#[repr(C)]
pub struct lws_token_limits {
    token_limit: [c_ushort; lws_token_indexes::WSI_TOKEN_COUNT as usize], /*< max chars for this token */
}

/** struct lws_protocol_vhost_options - linked list of per-vhost protocol
 * 					name=value options
 *
 * This provides a general way to attach a linked-list of name=value pairs,
 * which can also have an optional child link-list using the options member.
 */
#[repr(C)]
pub struct lws_protocol_vhost_options {
    next: *const lws_protocol_vhost_options,
    /**< linked list */
    options: *const lws_protocol_vhost_options,
    /**< child linked-list of more options for this node */
    name: *const c_char,
    /**< name of name=value pair */
    value: *const c_char, /*< value of name=value pair */
}

/** struct lws_extension -	An extension we support */
#[repr(C)]
pub struct lws_extension {
    name: *const c_char, /*< Formal extension name, eg, "permessage-deflate" */
    callback: lws_extension_callback_function, /*< Service callback */
    client_offer: *const c_char, /*< String containing exts and options client offers */

                         /* Add new things just above here ---^
                          * This is part of the ABI, don't needlessly break compatibility */
}

/** struct lws_http_mount
 *
 * arguments for mounting something in a vhost's url namespace
 */
#[repr(C)]
pub struct lws_http_mount {
    mount_next: *const lws_http_mount,
    /**< pointer to next struct lws_http_mount */
    mountpoint: *const c_char,
    /**< mountpoint in http pathspace, eg, "/" */
    origin: *const c_char,
    /**< path to be mounted, eg, "/var/www/warmcat.com" */
    def: *const c_char,
    /**< default target, eg, "index.html" */
    cprotocol: *const c_char,
    /**<"protocol-name" to handle mount */
    cgienv: *const lws_protocol_vhost_options,
    /**< optional linked-list of cgi options.  These are created
     * as environment variables for the cgi process
     */
    extra_mimetypes: *const lws_protocol_vhost_options,
    /**< optional linked-list of mimetype mappings */
    interpret: *const lws_protocol_vhost_options,
    /**< optional linked-list of files to be interpreted */
    cgi_timeout: c_int,
    /**< seconds cgi is allowed to live, if cgi://mount type */
    cache_max_age: c_int,
    /**< max-age for reuse of client cache of files, seconds */
    auth_mask: c_uint,
    /**< bits set here must be set for authorized client session */
    // unsigned int cache_reusable:1; /**< set if client cache may reuse this */
    // unsigned int cache_revalidate:1; /**< set if client cache should revalidate on use */
    // unsigned int cache_intermediaries:1; /**< set if intermediaries are allowed to cache */
    cache_reusable: c_uint,
    /**< set if client cache may reuse this */
    cache_revalidate: c_uint,
    /**< set if client cache should revalidate on use */
    cache_intermediaries: c_uint,
    /**< set if intermediaries are allowed to cache */
    origin_protocol: c_uchar,
    /**< one of enum lws_mount_protocols */
    mountpoint_len: c_uchar, /*< length of mountpoint string */
}

/** struct lws_context_creation_info - parameters to create context and /or vhost with
 *
 * This is also used to create vhosts.... if LWS_SERVER_OPTION_EXPLICIT_VHOSTS
 * is not given, then for backwards compatibility one vhost is created at
 * context-creation time using the info from this struct.
 *
 * If LWS_SERVER_OPTION_EXPLICIT_VHOSTS is given, then no vhosts are created
 * at the same time as the context, they are expected to be created afterwards.
 */
#[repr(C)]
pub struct lws_context_creation_info {
    pub port: c_int,
    /**< VHOST: Port to listen on... you can use CONTEXT_PORT_NO_LISTEN to
     * suppress listening on any port, that's what you want if you are
     * not running a websocket server at all but just using it as a
     * client */
    pub iface: *const c_char,
    /**< VHOST: NULL to bind the listen socket to all interfaces, or the
     * interface name, eg, "eth2"
     * If options specifies LWS_SERVER_OPTION_UNIX_SOCK, this member is
     * the pathname of a UNIX domain socket. you can use the UNIX domain
     * sockets in abstract namespace, by prepending an at symbol to the
     * socket name. */
    pub protocols: *const lws_protocols,
    /**< VHOST: Array of structures listing supported protocols and a protocol-
     * specific callback for each one.  The list is ended with an
     * entry that has a NULL callback pointer. */
    pub extensions: *const lws_extension,
    /**< VHOST: NULL or array of lws_extension structs listing the
     * extensions this context supports. */
    pub token_limits: *const lws_token_limits,
    /**< CONTEXT: NULL or struct lws_token_limits pointer which is initialized
     * with a token length limit for each possible WSI_TOKEN_ */
    pub ssl_private_key_password: *const c_char,
    /**< VHOST: NULL or the passphrase needed for the private key */
    pub ssl_cert_filepath: *const c_char,
    /**< VHOST: If libwebsockets was compiled to use ssl, and you want
     * to listen using SSL, set to the filepath to fetch the
     * server cert from, otherwise NULL for unencrypted */
    pub ssl_private_key_filepath: *const c_char,
    /**<  VHOST: filepath to private key if wanting SSL mode;
     * if this is set to NULL but sll_cert_filepath is set, the
     * OPENSSL_CONTEXT_REQUIRES_PRIVATE_KEY callback is called
     * to allow setting of the private key directly via openSSL
     * library calls */
    pub ssl_ca_filepath: *const c_char,
    /**< VHOST: CA certificate filepath or NULL */
    pub ssl_cipher_list: *const c_char,
    /**< VHOST: List of valid ciphers to use (eg,
     * "RC4-MD5:RC4-SHA:AES128-SHA:AES256-SHA:HIGH:!DSS:!aNULL"
     * or you can leave it as NULL to get "DEFAULT" */
    pub http_proxy_address: *const c_char,
    /**< VHOST: If non-NULL, attempts to proxy via the given address.
     * If proxy auth is required, use format "username:password\@server:port" */
    pub http_proxy_port: c_uint,
    /**< VHOST: If http_proxy_address was non-NULL, uses this port */
    pub gid: c_int,
    /**< CONTEXT: group id to change to after setting listen socket, or -1. */
    pub uid: c_int,
    /**< CONTEXT: user id to change to after setting listen socket, or -1. */
    pub options: c_uint,
    /**< VHOST + CONTEXT: 0, or LWS_SERVER_OPTION_... bitfields */
    pub user: *mut c_void,
    /**< CONTEXT: optional user pointer that can be recovered via the context
     *		pointer using lws_context_user */
    pub bka_time: c_int,
    /**< CONTEXT: 0 for no TCP keepalive, otherwise apply this keepalive
     * timeout to all libwebsocket sockets, client or server */
    pub ka_probes: c_int,
    /**< CONTEXT: if ka_time was nonzero, after the timeout expires how many
     * times to try to get a response from the peer before giving up
     * and killing the connection */
    pub ka_interval: c_int,
    /**< CONTEXT: if ka_time was nonzero, how long to wait before each ka_probes
     * attempt */
    // #ifdef LWS_OPENSSL_SUPPORT
    // 	SSL_CTX *provided_client_ssl_ctx;
    // 	/**< CONTEXT: If non-null, swap out libwebsockets ssl
    //  *		implementation for the one provided by provided_ssl_ctx.
    //  *		Libwebsockets no longer is responsible for freeing the context
    //  *		if this option is selected. */
    // #else /* maintain structure layout either way */
    // 	void *provided_client_ssl_ctx; /**< dummy if ssl disabled */
    // #endif
    pub max_http_header_data: c_short,
    /**< CONTEXT: The max amount of header payload that can be handled
     * in an http request (unrecognized header payload is dropped) */
    pub max_http_header_pool: c_short,
    /**< CONTEXT: The max number of connections with http headers that
     * can be processed simultaneously (the corresponding memory is
     * allocated for the lifetime of the context).  If the pool is
     * busy new incoming connections must wait for accept until one
     * becomes free. */
    pub count_threads: c_uint,
    /**< CONTEXT: how many contexts to create in an array, 0 = 1 */
    pub fd_limit_per_thread: c_uint,
    /**< CONTEXT: nonzero means restrict each service thread to this
     * many fds, 0 means the default which is divide the process fd
     * limit by the number of threads. */
    pub timeout_secs: c_uint,
    /**< VHOST: various processes involving network roundtrips in the
     * library are protected from hanging forever by timeouts.  If
     * nonzero, this member lets you set the timeout used in seconds.
     * Otherwise a default timeout is used. */
    ecdh_curve: *const c_char,
    /**< VHOST: if NULL, defaults to initializing server with "prime256v1" */
    vhost_name: *const c_char,
    /**< VHOST: name of vhost, must match external DNS name used to
     * access the site, like "warmcat.com" as it's used to match
     * Host: header and / or SNI name for SSL. */
    plugin_dirs: *const *const c_char,
    /**< CONTEXT: NULL, or NULL-terminated array of directories to
     * scan for lws protocol plugins at context creation time */
    pvo: *const lws_protocol_vhost_options,
    /**< VHOST: pointer to optional linked list of per-vhost
     * options made accessible to protocols */
    keepalive_timeout: c_int,
    /**< VHOST: (default = 0 = 60s) seconds to allow remote
     * client to hold on to an idle HTTP/1.1 connection */
    log_filepath: *const c_char,
    /**< VHOST: filepath to append logs to... this is opened before
     *		any dropping of initial privileges */
    mounts: *const lws_http_mount,
    /**< VHOST: optional linked list of mounts for this vhost */
    server_string: *const c_char,
    /**< CONTEXT: string used in HTTP headers to identify server
     *		software, if NULL, "libwebsockets". */
    pt_serv_buf_size: c_uint,
    /**< CONTEXT: 0 = default of 4096.  This buffer is used by
     * various service related features including file serving, it
     * defines the max chunk of file that can be sent at once.
     * At the risk of lws having to buffer failed large sends, it
     * can be increased to, eg, 128KiB to improve throughput. */
    max_http_header_data2: c_uint,
    /**< CONTEXT: if max_http_header_data is 0 and this
     * is nonzero, this will be used in place of the default.  It's
     * like this for compatibility with the original short version,
     * this is unsigned int length. */
    ssl_options_set: c_long,
    /**< VHOST: Any bits set here will be set as SSL options */
    ssl_options_clear: c_long,
    /**< VHOST: Any bits set here will be cleared as SSL options */
    ws_ping_pong_interval: c_ushort,
    /**< CONTEXT: 0 for none, else interval in seconds between sending
     * PINGs on idle websocket connections.  When the PING is sent,
     * the PONG must come within the normal timeout_secs timeout period
     * or the connection will be dropped.
     * Any RX or TX traffic on the connection restarts the interval timer,
     * so a connection which always sends or receives something at intervals
     * less than the interval given here will never send PINGs / expect
     * PONGs.  Conversely as soon as the ws connection is established, an
     * idle connection will do the PING / PONG roundtrip as soon as
     * ws_ping_pong_interval seconds has passed without traffic
     */
    headers: *const lws_protocol_vhost_options,
    /**< VHOST: pointer to optional linked list of per-vhost
     * canned headers that are added to server responses */
    /* Add new things just above here ---^
     * This is part of the ABI, don't needlessly break compatibility
     *
     * The below is to ensure later library versions with new
     * members added above will see 0 (default) even if the app
     * was not built against the newer headers.
     */

    //void *_unused[8]; /**< dummy */
    _unused: [*mut c_void; 8],
}

/** struct lws_client_connect_info - parameters to connect with when using
 *				    lws_client_connect_via_info() */

#[repr(C)]
pub struct lws_client_connect_info {
    pub context: *mut c_void,
    /**< lws context to create connection in */
    pub address: *const c_char,
    /**< remote address to connect to */
    pub port: c_int,
    /**< remote port to connect to */
    pub ssl_connection: c_int,
    /**< nonzero for ssl */
    pub path: *const c_char,
    /**< uri path */
    pub host: *const c_char,
    /**< content of host header */
    pub origin: *const c_char,
    /**< content of origin header */
    pub protocol: *const c_char,
    /**< list of ws protocols we could accept */
    pub ietf_version_or_minus_one: c_int,
    /**< deprecated: currently leave at 0 or -1 */
    pub userdata: *mut c_void,
    /**< if non-NULL, use this as wsi user_data instead of malloc it */
    pub client_exts: *const lws_extension,
    /**< array of extensions that may be used on connection */
    pub method: *const c_char,
    /**< if non-NULL, do this http method instead of ws[s] upgrade.
     * use "GET" to be a simple http client connection */
    pub parent_wsi: *mut c_void,
    /**< if another wsi is responsible for this connection, give it here.
     * this is used to make sure if the parent closes so do any
     * child connections first. */
    pub uri_replace_from: *const c_char,
    /**< if non-NULL, when this string is found in URIs in
     * text/html content-encoding, it's replaced with uri_replace_to */
    pub uri_replace_to: *const c_char,
    /**< see uri_replace_from */
    pub vhost: *mut c_void,
    /**< vhost to bind to (used to determine related SSL_CTX) */
    pub pwsi: *mut *mut c_void,
    /**< if not NULL, store the new wsi here early in the connection
     * process.  Although we return the new wsi, the call to create the
     * client connection does progress the connection somewhat and may
     * meet an error that will result in the connection being scrubbed and
     * NULL returned.  While the wsi exists though, he may process a
     * callback like CLIENT_CONNECTION_ERROR with his wsi: this gives the
     * user callback a way to identify which wsi it is that faced the error
     * even before the new wsi is returned and even if ultimately no wsi
     * is returned.
     */
    /* Add new things just above here ---^
     * This is part of the ABI, don't needlessly break compatibility
     *
     * The below is to ensure later library versions with new
     * members added above will see 0 (default) even if the app
     * was not built against the newer headers.
     */

    // void *_unused[4]; /**< dummy */
    pub _unused: [*mut c_void; 4],
}

/*
 * NOTE: These public enums are part of the abi.  If you want to add one,
 * add it at where specified so existing users are unaffected.
 */
#[repr(C)]
pub enum lws_write_protocol {
    LWS_WRITE_TEXT = 0,
    /**< Send a ws TEXT message,the pointer must have LWS_PRE valid
     * memory behind it.  The receiver expects only valid utf-8 in the
     * payload */
    LWS_WRITE_BINARY = 1,
    /**< Send a ws BINARY message, the pointer must have LWS_PRE valid
     * memory behind it.  Any sequence of bytes is valid */
    LWS_WRITE_CONTINUATION = 2,
    /**< Continue a previous ws message, the pointer must have LWS_PRE valid
     * memory behind it */
    LWS_WRITE_HTTP = 3,
    /**< Send HTTP content */
    /* LWS_WRITE_CLOSE is handled by lws_close_reason() */
    LWS_WRITE_PING = 5,
    LWS_WRITE_PONG = 6,

    /* Same as write_http but we know this write ends the transaction */
    LWS_WRITE_HTTP_FINAL = 7,

    /* HTTP2 */
    LWS_WRITE_HTTP_HEADERS = 8,
    /**< Send http headers (http2 encodes this payload and LWS_WRITE_HTTP
     * payload differently, http 1.x links also handle this correctly. so
     * to be compatible with both in the future,header response part should
     * be sent using this regardless of http version expected)
     */
    /****** add new things just above ---^ ******/
    /* flags */
    LWS_WRITE_NO_FIN = 0x40,
    /**< This part of the message is not the end of the message */
    LWS_WRITE_CLIENT_IGNORE_XOR_MASK = 0x80, /*< client packet payload goes out on wire unmunged
                                              * only useful for security tests since normal servers cannot
                                              * decode the content if used */
}

#[doc = "< tls_gpio_dir output"]
pub const WM_GPIO_DIR_OUTPUT: tls_gpio_dir = 0;
#[doc = "< tls_gpio_dir input"]
pub const WM_GPIO_DIR_INPUT: tls_gpio_dir = 1;
#[doc = "< Indicating gpio direction"]
pub type tls_gpio_dir = i32;
#[doc = "< tls_gpio_attr floating status"]
pub const WM_GPIO_ATTR_FLOATING: tls_gpio_attr = 0;
#[doc = "< tls_gpio_attr pull high"]
pub const WM_GPIO_ATTR_PULLHIGH: tls_gpio_attr = 1;
#[doc = "< tls_gpio_attrpull low"]
pub const WM_GPIO_ATTR_PULLLOW: tls_gpio_attr = 2;
#[doc = "< Indicating gpio attribute"]
pub type tls_gpio_attr = i32;

#[doc = "< gpio a0"]
pub const WM_IO_PA_00: tls_io_name = 0;
#[doc = "< gpio a1"]
pub const WM_IO_PA_01: tls_io_name = 1;
#[doc = "< gpio a2"]
pub const WM_IO_PA_02: tls_io_name = 2;
#[doc = "< gpio a3"]
pub const WM_IO_PA_03: tls_io_name = 3;
#[doc = "< gpio a4"]
pub const WM_IO_PA_04: tls_io_name = 4;
#[doc = "< gpio a5"]
pub const WM_IO_PA_05: tls_io_name = 5;
#[doc = "< gpio a6"]
pub const WM_IO_PA_06: tls_io_name = 6;
#[doc = "< gpio a7"]
pub const WM_IO_PA_07: tls_io_name = 7;
#[doc = "< gpio a8"]
pub const WM_IO_PA_08: tls_io_name = 8;
#[doc = "< gpio a9"]
pub const WM_IO_PA_09: tls_io_name = 9;
#[doc = "< gpio a10"]
pub const WM_IO_PA_10: tls_io_name = 10;
#[doc = "< gpio a11"]
pub const WM_IO_PA_11: tls_io_name = 11;
#[doc = "< gpio a12"]
pub const WM_IO_PA_12: tls_io_name = 12;
#[doc = "< gpio a13"]
pub const WM_IO_PA_13: tls_io_name = 13;
#[doc = "< gpio a14"]
pub const WM_IO_PA_14: tls_io_name = 14;
#[doc = "< gpio a15"]
pub const WM_IO_PA_15: tls_io_name = 15;
#[doc = "< gpio b0"]
pub const WM_IO_PB_00: tls_io_name = 16;
#[doc = "< gpio b1"]
pub const WM_IO_PB_01: tls_io_name = 17;
#[doc = "< gpio b2"]
pub const WM_IO_PB_02: tls_io_name = 18;
#[doc = "< gpio b3"]
pub const WM_IO_PB_03: tls_io_name = 19;
#[doc = "< gpio b4"]
pub const WM_IO_PB_04: tls_io_name = 20;
#[doc = "< gpio b5"]
pub const WM_IO_PB_05: tls_io_name = 21;
#[doc = "< gpio b6"]
pub const WM_IO_PB_06: tls_io_name = 22;
#[doc = "< gpio b7"]
pub const WM_IO_PB_07: tls_io_name = 23;
#[doc = "< gpio b8"]
pub const WM_IO_PB_08: tls_io_name = 24;
#[doc = "< gpio b9"]
pub const WM_IO_PB_09: tls_io_name = 25;
#[doc = "< gpio b10"]
pub const WM_IO_PB_10: tls_io_name = 26;
#[doc = "< gpio b11"]
pub const WM_IO_PB_11: tls_io_name = 27;
#[doc = "< gpio b12"]
pub const WM_IO_PB_12: tls_io_name = 28;
#[doc = "< gpio b13"]
pub const WM_IO_PB_13: tls_io_name = 29;
#[doc = "< gpio b14"]
pub const WM_IO_PB_14: tls_io_name = 30;
#[doc = "< gpio b15"]
pub const WM_IO_PB_15: tls_io_name = 31;
#[doc = "< gpio b16"]
pub const WM_IO_PB_16: tls_io_name = 32;
#[doc = "< gpio b17"]
pub const WM_IO_PB_17: tls_io_name = 33;
#[doc = "< gpio b18"]
pub const WM_IO_PB_18: tls_io_name = 34;
#[doc = "< gpio b19"]
pub const WM_IO_PB_19: tls_io_name = 35;
#[doc = "< gpio b20"]
pub const WM_IO_PB_20: tls_io_name = 36;
#[doc = "< gpio b21"]
pub const WM_IO_PB_21: tls_io_name = 37;
#[doc = "< gpio b22"]
pub const WM_IO_PB_22: tls_io_name = 38;
#[doc = "< gpio b23"]
pub const WM_IO_PB_23: tls_io_name = 39;
#[doc = "< gpio b24"]
pub const WM_IO_PB_24: tls_io_name = 40;
#[doc = "< gpio b25"]
pub const WM_IO_PB_25: tls_io_name = 41;
#[doc = "< gpio b26"]
pub const WM_IO_PB_26: tls_io_name = 42;
#[doc = "< gpio b27"]
pub const WM_IO_PB_27: tls_io_name = 43;
#[doc = "< gpio b28"]
pub const WM_IO_PB_28: tls_io_name = 44;
#[doc = "< gpio b29"]
pub const WM_IO_PB_29: tls_io_name = 45;
#[doc = "< gpio b30"]
pub const WM_IO_PB_30: tls_io_name = 46;
#[doc = "< gpio b31"]
pub const WM_IO_PB_31: tls_io_name = 47;
#[doc = " io name"]
pub type tls_io_name = i32;

extern "C" {
    /*
     *********************************************************************************************************
     *                                       DELAY TASK 'n' TICKS
     *
     * Description: This function is called to delay execution of the currently running task until the
     *              specified number of system ticks expires.  This, of course, directly equates to delaying
     *              the current task for some time to expire.  No delay will result If the specified delay is
     *              0.  If the specified delay is greater than 0 then, a context switch will result.
     *
     * Arguments  : ticks     is the time delay that the task will be suspended in number of clock 'ticks'.
     *                        Note that by specifying 0, the task will not be delayed.
     *
     * Returns    : none
     *********************************************************************************************************
     */
    pub fn tls_os_time_delay(time: u32);

    //pub unsafe extern fn printf(format: *const c_char, ...) -> c_int

    // pub fn mem_alloc_debug(size: u32) -> *mut u8;
    // pub fn mem_free_debug(p: *mut u8);
    // pub fn mem_realloc_debug(mem_address: *mut u8, size:u32) -> *mut u8;
    // pub fn mem_calloc_debug(u32 length, u32 size) -> *mut c_void;

    /*
     *********************************************************************************************************
     *                                         GET CURRENT SYSTEM TIME
     *
     * Description: This function is used by your application to obtain the current value of the 32-bit
     *              counter which keeps track of the number of clock ticks.
     *
     * Arguments  : none
     *
     * Returns    : The current value of OSTime
     *********************************************************************************************************
     */
    pub fn tls_os_get_time() -> u32;

    /*
     *********************************************************************************************************
     *                                     CREATE A TASK (Extended Version)
     *
     * Description: This function is used to have uC/OS-II manage the execution of a task.  Tasks can either
     *              be created prior to the start of multitasking or by a running task.  A task cannot be
     *              created by an ISR.
     *
     * Arguments  : task      is a pointer to the task'
     *
     *			name 	is the task's name
     *
     *			entry	is the task's entry function
     *
     *              param     is a pointer to an optional data area which can be used to pass parameters to
     *                        the task when the task first executes.  Where the task is concerned it thinks
     *                        it was invoked and passed the argument 'param' as follows:
     *
     *                            void Task (void *param)
     *                            {
     *                                for (;;) {
     *                                    Task code;
     *                                }
     *                            }
     *
     *              stk_start      is a pointer to the task's bottom of stack.
     *
     *              stk_size  is the size of the stack in number of elements.  If OS_STK is set to u8,
     *                        'stk_size' corresponds to the number of bytes available.  If OS_STK is set to
     *                        INT16U, 'stk_size' contains the number of 16-bit entries available.  Finally, if
     *                        OS_STK is set to INT32U, 'stk_size' contains the number of 32-bit entries
     *                        available on the stack.
     *
     *              prio      is the task's priority.  A unique priority MUST be assigned to each task and the
     *                        lower the number, the higher the priority.
     *
     *              flag       contains additional information about the behavior of the task.
     *
     * Returns    : TLS_OS_SUCCESS             if the function was successful.
     *              TLS_OS_ERROR
     *********************************************************************************************************
     */
    pub fn tls_os_task_create(
        task: *mut tls_os_task_t,
        name: *const c_char,
        entry: extern "C" fn(param: *mut c_void),
        param: *mut c_void,
        stk_start: *mut u8,
        stk_size: u32,
        prio: u32,
        flag: u32,
    ) -> tls_os_status_t;

    /*
     *********************************************************************************************************
     *                                            DELETE A TASK
     *
     * Description: This function allows you to delete a task.  The calling task can delete itself by
     *              its own priority number.  The deleted task is returned to the dormant state and can be
     *              re-activated by creating the deleted task again.
     *
     * Arguments  : prio: the task priority
     *                    freefun: function to free resource
     *
     * Returns    : TLS_OS_SUCCESS             if the call is successful
     *             	 TLS_OS_ERROR
     *********************************************************************************************************
     */
    pub fn tls_os_task_del(prio: u8, freefun: extern "C" fn()) -> tls_os_status_t;

    /*
     *********************************************************************************************************
     *                                        POST MESSAGE TO A QUEUE
     *
     * Description: This function sends a message to a queue
     *
     * Arguments  : queue        is a pointer to the event control block associated with the desired queue
     *
     *              	msg          is a pointer to the message to send.
     *
     *			msg_size
     * Returns    : TLS_OS_SUCCESS
     *			TLS_OS_ERROR
     *********************************************************************************************************
     */
    pub fn tls_os_queue_send(
        queue: *mut tls_os_queue_t,
        msg: *mut c_void,
        msg_size: u32,
    ) -> tls_os_status_t;

    /*
     *********************************************************************************************************
     *                                     PEND ON A QUEUE FOR A MESSAGE
     *
     * Description: This function waits for a message to be sent to a queue
     *
     * Arguments  : queue        is a pointer to the event control block associated with the desired queue
     *
     *			msg		is a pointer to the message received
     *
     *			msg_size
     *
     *              wait_time       is an optional timeout period (in clock ticks).  If non-zero, your task will
     *                            wait for a message to arrive at the queue up to the amount of time
     *                            specified by this argument.  If you specify 0, however, your task will wait
     *                            forever at the specified queue or, until a message arrives.
     *
     * Returns    : TLS_OS_SUCCESS
     *			TLS_OS_ERROR
     *********************************************************************************************************
     */
    pub fn tls_os_queue_receive(
        queue: *mut tls_os_queue_t,
        msg: *mut *mut c_void,
        msg_size: u32,
        wait_time: u32,
    ) -> tls_os_status_t;

    /*
     *********************************************************************************************************
     *                                        CREATE A MESSAGE QUEUE
     *
     * Description: This function creates a message queue if free event control blocks are available.
     *
     * Arguments  : queue	is a pointer to the event control clock (OS_EVENT) associated with the
     *                                created queue
     *
     *			queue_start         is a pointer to the base address of the message queue storage area.  The
     *                            storage area MUST be declared as an array of pointers to 'void' as follows
     *
     *                            void *MessageStorage[size]
     *
     *              	queue_size          is the number of elements in the storage area
     *
     *			msg_size
     *
     * Returns    : TLS_OS_SUCCESS
     *			TLS_OS_ERROR
     *********************************************************************************************************
     */

    pub fn tls_os_queue_create(queue: *mut *mut tls_os_queue_t, queue_size: u32)
        -> tls_os_status_t;

    /*
     *********************************************************************************************************
     *                                        DELETE A MESSAGE QUEUE
     *
     * Description: This function deletes a message queue and readies all tasks pending on the queue.
     *
     * Arguments  : queue        is a pointer to the event control block associated with the desired
     *                            queue.
     *
     *
     * Returns    : TLS_OS_SUCCESS
     *			TLS_OS_ERROR
     *********************************************************************************************************
     */
    pub fn tls_os_queue_delete(queue: *mut tls_os_queue_t) -> tls_os_status_t;

    /**
     * @brief          This function is used to write data to the flash.
     *
     * @param[in]      addr     is byte offset addr for write to the flash
     * @param[in]      buf       is the data buffer want to write to flash
     * @param[in]      len       is the byte length want to write
     *
     * @retval         TLS_FLS_STATUS_OK	           if write flash success
     * @retval         TLS_FLS_STATUS_EPERM	    if flash struct point is null
     * @retval         TLS_FLS_STATUS_ENODRV	    if flash driver is not installed
     * @retval         TLS_FLS_STATUS_EINVAL	    if argument is invalid
     * @retval         TLS_FLS_STATUS_EIO           if io error
     *
     * @note           None
     */
    pub fn tls_fls_write(addr: u32, buf: *const u8, len: u32) -> i32;

    /**
     * @brief          This function is used to read data from the flash.
     *
     * @param[in]      addr                 Specifies the starting address to read from.
     * @param[in]      buf                  Specified the address to save the readback data.
     * @param[in]      len                  Specifies the length of the data to read.
     *
     * @retval         TLS_FLS_STATUS_OK	    if read sucsess
     * @retval	   	   TLS_FLS_STATUS_EPERM		if flash driver module not beed installed
     * @retval		   TLS_FLS_STATUS_EINVAL    if arguments invalid
     * @retval         TLS_FLS_STATUS_EIO	    if read fail
     *
     * @note           None
     */
    pub fn tls_fls_read(addr: u32, buf: *mut u8, len: u32) -> i32;

    /**
     * @brief          This function is used to create the timer
     *
     * @param[in]      cfg     timer configuration
     *
     * @retval         	WM_TIMER_ID_INVALID     failed
     * @retval         	other                   timer id[0~5]
     *
     * @note
     * user not need clear interrupt flag.
     * timer callback function is called in interrupt,
     * so can not operate the critical data in the callback fuuction,
     * recommendation to send messages to other tasks to operate it.
     */
    pub fn tls_timer_create(cfg: *mut tls_timer_cfg) -> u8;

    /**
     * @brief          This function is used to start the timer
     *
     * @param[in]      	timer_id    timer id[0~5]
     *
     * @return         None
     *
     * @note           None
     */
    pub fn tls_timer_start(timer_id: u8);

    /**
     * @brief          This function is used to stop the timer
     *
     * @param[in]      	timer_id    timer id[0~5]
     *
     * @return         None
     *
     * @note           None
     */
    pub fn tls_timer_stop(timer_id: u8);

    /**
     * @brief           This function is used to change a timer wait time
     *
     * @param[in]      	timer_id    timer id[0~5]
     *
     * @param[in]      	newtime     new wait time
     *
     * @retval         	None
     *
     * @note            If the timer does not start, this function will start the timer
     */
    pub fn tls_timer_change(timer_id: u8, newtime: u32);

    /**
     * @brief          This function is used to delete the timer
     *
     * @param[in]      	timer_id    timer id[0~5]
     *
     * @return         None
     *
     * @note           None
     */
    pub fn tls_timer_destroy(timer_id: u8);

    /*
     ************************************************************************************************************************
     *                                                   CREATE A TIMER
     *
     * Description: This function is called by your application code to create a timer.
     *
     * Arguments  : timer	A pointer to an OS_TMR data structure.This is the 'handle' that your application
     *						will use to reference the timer created.
     *
     *		        callback      Is a pointer to a callback function that will be called when the timer expires.  The
     *                               callback function must be declared as follows:
     *
     *                               void MyCallback (OS_TMR *ptmr, void *p_arg);
     *
     * 	             callback_arg  Is an argument (a pointer) that is passed to the callback function when it is called.
     *
     *          	   	 period        The 'period' being repeated for the timer.
     *                               If you specified 'OS_TMR_OPT_PERIODIC' as an option, when the timer expires, it will
     *                               automatically restart with the same period.
     *
     *			repeat	if repeat
     *
     *             	pname         Is a pointer to an ASCII string that is used to name the timer.  Names are useful for
     *                               debugging.
     *
     *Returns    : TLS_OS_SUCCESS
     *			TLS_OS_ERROR
     ************************************************************************************************************************
     */
    pub fn tls_os_timer_create(
        timer: *mut *mut c_void,
        callback: extern "C" fn(ptmr: *mut c_void, parg: *mut c_void),
        callback_arg: *mut c_void,
        period: u32,
        repeat: bool,
        name: *const c_char,
    ) -> tls_os_status_t;

    /*
     ************************************************************************************************************************
     *                                                   START A TIMER
     *
     * Description: This function is called by your application code to start a timer.
     *
     * Arguments  : timer          Is a pointer to an OS_TMR
     *
     ************************************************************************************************************************
     */
    pub fn tls_os_timer_start(timer: *mut c_void);

    /*
     ************************************************************************************************************************
     *                                                   STOP A TIMER
     *
     * Description: This function is called by your application code to stop a timer.
     *
     * Arguments  : timer          Is a pointer to the timer to stop.
     *
     ************************************************************************************************************************
     */
    pub fn tls_os_timer_stop(timer: *mut c_void);

    pub fn tls_os_timer_delete(timer: *mut c_void) -> tls_os_status_t;

    /**
     * @brief          This function is used to set one system parameter by its id
     *
     * @param[in]      id         param id,from TLS_PARAM_ID_SSID
                                 to (TLS_PARAM_ID_MAX - 1)
    * @param[in]      *argv      store parameters
    * @param[in]      to_flash   whether the parameter is written to flash,
                                1:write to flash,0:only write memory
    *
    * @retval         TLS_PARAM_STATUS_OK       set success
    * @retval         TLS_PARAM_STATUS_EINVALID invalid param
    *
    * @note           None
    */
    pub fn tls_param_set(id: i32, argv: *mut c_void, to_flash: bool) -> i32;

    /**********************************************************************************************************
    * Description: 	This function is used to get system parameter.
    *
    * Arguments  : 	id		param id,from TLS_PARAM_ID_SSID to (TLS_PARAM_ID_MAX - 1)
    *				argc		store parameters
    *				from_flash	whether the parameter is readed from flash,1 read from flash

    * Returns    :		TLS_PARAM_STATUS_OK	success
    *				TLS_PARAM_STATUS_EINVALID	invalid param
    **********************************************************************************************************/
    pub fn tls_param_get(id: i32, argv: *mut c_void, from_flash: bool) -> i32;

    /**
     * @brief          This function is used to destroy or leave Wi-Fi network
     *
     * @param          None
     *
     * @return         None
     *
     * @note           For AP,   destroy soft AP
     *                 For STA,  leave the network by AP
     *                 For IBSS, destroy or leave the IBSS network.
     */
    pub fn tls_wifi_disconnect();

    /**
     * @brief          This function is used to connect AP
     *
     * @param[in]      *ssid       Network  Name to connect
     * @param[in]      ssid_len    length of SSID
     * @param[in]      *pwd        password to connect AP
     * @param[in]      pwd_len     length of password
     *
     * @retval         WM_SUCCESS    config ok, wifi will start to connect AP;
     * @retval         WM_FAILED     parameter wrong
     *
     * @note           User should register Wi-Fi status callback function
     *                 to get result;
     *				   wifi_status_change_cb just return WIFI MAC layer status;
     *				   User should register netif status callback
     *                 to get TCP/IP layer status;
     */
    pub fn tls_wifi_connect(ssid: *const u8, ssid_len: u8, pwd: *const u8, pwd_len: u8) -> i32;

    /**
     * @brief          This function is used to destroy soft ap
     *
     * @param          None
     *
     * @return         None
     *
     * @note           None
     */
    pub fn tls_wifi_softap_destroy();

    /***************************************************************************
     * Function: tls_wifi_set_oneshot_flag
     *
     * Description: This function is used to set oneshot flag.
     *
     * Input: flag 0:one shot  closed
     * 		      1:one shot  open
     * Output: None
     *
     * Return: None
     *
     * Date : 2014-6-11
     ****************************************************************************/
    pub fn tls_wifi_set_oneshot_flag(flag: u8);

    /**
     * @brief          This function is used to get current Wi-Fi State
     *
     * @param          None
     *
     * @retval         Wi-Fi State(enum)
     *
     * @note           None
     */
    pub fn tls_wifi_get_state() -> tls_wifi_states;

    pub fn tls_netif_add_status_event(event_fn: extern "C" fn(status: u8)) -> u8;

    /**
     * @brief          This function is used to get IP information stored in
                     tls_ethif struct
    *
    * @param[in]      None
    *
    * @retval         tls_ethif *     Pointer to struct tls_ethif
    *
    * @note           None
    */
    pub fn tls_netif_get_ethif() -> *mut tls_ethif;

    // pub fn print_ipaddr(ip: *const ip_addr_t);

    //----------------- websocket ---------------------------------
    pub fn lws_create_context(info: *mut lws_context_creation_info) -> *mut c_void;
    pub fn lws_client_connect_via_info(i: *mut lws_client_connect_info) -> *mut c_void;
    pub fn lws_context_destroy(context: *mut c_void);
    pub fn lws_callback_on_writable(wsi: *mut c_void) -> c_int;
    pub fn lws_service(context: *mut c_void, timeout_ms: c_int) -> c_int;
    pub fn lws_write(
        wsi: *mut c_void,
        buf: *mut c_uchar,
        len: size_t,
        wp: lws_write_protocol,
    ) -> c_int;

    // gpio
    pub fn tls_gpio_cfg(gpio_pin: tls_io_name, dir: tls_gpio_dir, attr: tls_gpio_attr);
    pub fn tls_gpio_write(gpio_pin: tls_io_name, value: u8);

    // pwm
    /**
     * @brief  config the pins used for pwm1
     * @param  io_name: config pwm1 pins name
     *			WM_IO_PA_00
     *			WM_IO_PB_18
     *			WM_IO_PB_05
     *			WM_IO_PA_05
     *			WM_IO_PB_19
     *			WM_IO_PB_30
     *
     * @return None
     */
    pub fn wm_pwm1_config(io_name: tls_io_name);

    /**
     * @brief  config the pins used for pwm2
     * @param  io_name: config pwm2 pins name
     *			WM_IO_PA_01
     *			WM_IO_PB_17
     *			WM_IO_PB_04
     *			WM_IO_PA_07
     *			WM_IO_PB_13
     *			WM_IO_PB_20
     *
     * @return None
     */
    pub fn wm_pwm2_config(io_name: tls_io_name);

    /**
     * @brief  config the pins used for pwm3
     * @param  io_name: config pwm3 pins name
     *			WM_IO_PA_02
     *			WM_IO_PB_16
     *			WM_IO_PB_03
     *			WM_IO_PA_08
     *			WM_IO_PB_21
     *
     * @return None
     */
    pub fn wm_pwm3_config(io_name: tls_io_name);

    /**
     * @brief  config the pins used for pwm4
     * @param  io_name: config pwm4 pins name
     *			WM_IO_PA_03
     *			WM_IO_PB_15
     *			WM_IO_PB_02
     *			WM_IO_PA_09
     *			WM_IO_PB_22
     *			WM_IO_PB_06
     *
     * @return None
     */
    pub fn wm_pwm4_config(io_name: tls_io_name);

    /**
     * @brief  config the pins used for pwm5
     * @param  io_name: config pwm5 pins name
     *			WM_IO_PA_04
     *			WM_IO_PB_14
     *			WM_IO_PB_01
     *			WM_IO_PA_10
     *			WM_IO_PB_23
     *			WM_IO_PB_08
     *
     * @return None
     */
    pub fn wm_pwm5_config(io_name: tls_io_name);

    /**
     * @brief          This function is used to initial pwm
     *
     * @param[in]      channel    pwm channel, range from 0 to 4
     * @param[in]      freq       is a pointer to frequency, freq range from 1 to 156250
     * @param[in]      duty       is a pointer to duty radio, duty range from 0 to 255
     * @param[in]      pnum       period num,range from 0 to 255
     *
     * @retval         WM_SUCCESS success
     * @retval         WM_FAILED  failed
     *
     * @note           None
     */
    pub fn tls_pwm_init(channel: u8, freq: u32, duty: u8, pnum: u8) -> i32;

    /**
     * @brief          This function is used to start pwm
     *
     * @param[in]      channel    pwm channel, range from 0 to 4
     *
     * @retval         WM_SUCCESS success
     * @retval         WM_FAILED  failed
     *
     * @note           None
     */
    pub fn tls_pwm_start(channel: u8) -> i32;

    /**
     * @brief          This function is used to set duty radio
     *
     * @param[in]      channel    pwm channel NO., range form 0 to 4
     * @param[in]      duty       duty radio, range from 0 to 255
     *
     * @return         None
     *
     * @note           None
     */
    pub fn tls_pwm_duty_set(channel: u8, duty: u8);

    /**
     * @brief          This function is used to stop pwm
     *
     * @param[in]      channel    pwm channel, range from 0 to 4
     *
     * @retval         WM_SUCCESS success
     * @retval         WM_FAILED  failed
     *
     * @note           None
     */
    pub fn tls_pwm_stop(channel: u8);
}
