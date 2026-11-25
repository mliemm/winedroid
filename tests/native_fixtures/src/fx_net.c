#define _GNU_SOURCE 1
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <poll.h>
#include <stdint.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <sys/select.h>
#include <sys/socket.h>
#include <sys/timerfd.h>
#include <time.h>
#include <unistd.h>

static int loopback_address(struct sockaddr_in *address)
{
    memset(address, 0, sizeof(*address));
    address->sin_family = AF_INET;
    address->sin_addr.s_addr = 0x0100007fu;
    address->sin_port = 0;
    return 0;
}

int fx_tcp_roundtrip(void)
{
    int listener = socket(AF_INET, SOCK_STREAM, 0);
    if (listener < 0) return -1;
    struct sockaddr_in address;
    loopback_address(&address);
    if (bind(listener, (struct sockaddr *)&address, sizeof(address)) != 0) {
        close(listener);
        return -2;
    }
    if (listen(listener, 4) != 0) {
        close(listener);
        return -3;
    }
    socklen_t length = sizeof(address);
    if (getsockname(listener, (struct sockaddr *)&address, &length) != 0) {
        close(listener);
        return -4;
    }
    int client = socket(AF_INET, SOCK_STREAM, 0);
    if (client < 0) {
        close(listener);
        return -5;
    }
    if (connect(client, (struct sockaddr *)&address, sizeof(address)) != 0) {
        close(client);
        close(listener);
        return -6;
    }
    int server = accept(listener, NULL, NULL);
    if (server < 0) {
        close(client);
        close(listener);
        return -7;
    }
    int mask = 0;
    if (send(client, "ping", 4, 0) == 4) mask |= 1;
    char request[8] = {0};
    if (recv(server, request, 4, 0) == 4 && memcmp(request, "ping", 4) == 0) mask |= 2;
    if (send(server, "pong", 4, 0) == 4) mask |= 4;
    char reply[8] = {0};
    if (recv(client, reply, 4, 0) == 4 && memcmp(reply, "pong", 4) == 0) mask |= 8;
    close(server);
    close(client);
    close(listener);
    return mask;
}

int fx_epoll_level(void)
{
    int ends[2];
    if (pipe(ends) != 0) return -1;
    int queue = epoll_create1(0);
    if (queue < 0) {
        close(ends[0]);
        close(ends[1]);
        return -2;
    }
    struct epoll_event interest;
    memset(&interest, 0, sizeof(interest));
    interest.events = EPOLLIN;
    if (epoll_ctl(queue, EPOLL_CTL_ADD, ends[0], &interest) != 0) return -3;
    char byte = 'a';
    if (write(ends[1], &byte, 1) != 1) return -4;
    struct epoll_event events[4];
    int mask = 0;
    if (epoll_wait(queue, events, 4, 1000) == 1 && (events[0].events & EPOLLIN)) mask |= 1;
    if (epoll_wait(queue, events, 4, 1000) == 1 && (events[0].events & EPOLLIN)) mask |= 2;
    char sink;
    if (read(ends[0], &sink, 1) != 1) return -5;
    if (epoll_wait(queue, events, 4, 0) == 0) mask |= 4;
    close(queue);
    close(ends[0]);
    close(ends[1]);
    return mask;
}

int fx_epoll_edge(void)
{
    int ends[2];
    if (pipe(ends) != 0) return -1;
    int queue = epoll_create1(0);
    if (queue < 0) return -2;
    struct epoll_event interest;
    memset(&interest, 0, sizeof(interest));
    interest.events = EPOLLIN | EPOLLET;
    if (epoll_ctl(queue, EPOLL_CTL_ADD, ends[0], &interest) != 0) return -3;
    char byte = 'a';
    if (write(ends[1], &byte, 1) != 1) return -4;
    struct epoll_event events[4];
    int mask = 0;
    if (epoll_wait(queue, events, 4, 1000) == 1) mask |= 1;
    if (epoll_wait(queue, events, 4, 0) == 0) mask |= 2;
    if (write(ends[1], &byte, 1) != 1) return -5;
    if (epoll_wait(queue, events, 4, 1000) == 1) mask |= 4;
    close(queue);
    close(ends[0]);
    close(ends[1]);
    return mask;
}

int fx_epoll_oneshot(void)
{
    int ends[2];
    if (pipe(ends) != 0) return -1;
    int queue = epoll_create1(0);
    if (queue < 0) return -2;
    struct epoll_event interest;
    memset(&interest, 0, sizeof(interest));
    interest.events = EPOLLIN | EPOLLONESHOT;
    if (epoll_ctl(queue, EPOLL_CTL_ADD, ends[0], &interest) != 0) return -3;
    char byte = 'a';
    if (write(ends[1], &byte, 1) != 1) return -4;
    struct epoll_event events[4];
    int mask = 0;
    if (epoll_wait(queue, events, 4, 1000) == 1) mask |= 1;
    if (epoll_wait(queue, events, 4, 100) == 0) mask |= 2;
    if (epoll_ctl(queue, EPOLL_CTL_MOD, ends[0], &interest) != 0) return -5;
    if (epoll_wait(queue, events, 4, 1000) == 1) mask |= 4;
    close(queue);
    close(ends[0]);
    close(ends[1]);
    return mask;
}

int fx_eventfd_value(void)
{
    int bell = eventfd(0, 0);
    if (bell < 0) return -errno;
    uint64_t value = 7;
    if (write(bell, &value, 8) != 8) return -2;
    uint64_t received = 0;
    if (read(bell, &received, 8) != 8) return -3;
    close(bell);
    return (int)received;
}

int fx_pipe_poll(void)
{
    int ends[2];
    if (pipe(ends) != 0) return -1;
    char byte = 'p';
    if (write(ends[1], &byte, 1) != 1) return -2;
    struct pollfd request = {.fd = ends[0], .events = POLLIN, .revents = 0};
    int mask = 0;
    if (poll(&request, 1, 1000) == 1 && (request.revents & POLLIN)) mask |= 1;
    char sink = 0;
    if (read(ends[0], &sink, 1) == 1 && sink == 'p') mask |= 2;
    close(ends[0]);
    close(ends[1]);
    return mask;
}

int fx_select_readable(void)
{
    int ends[2];
    if (pipe(ends) != 0) return -1;
    char byte = 's';
    if (write(ends[1], &byte, 1) != 1) return -2;
    fd_set read;
    FD_ZERO(&read);
    FD_SET(ends[0], &read);
    struct timeval timeout = {.tv_sec = 1, .tv_usec = 0};
    int mask = 0;
    if (select(ends[0] + 1, &read, NULL, NULL, &timeout) == 1 && FD_ISSET(ends[0], &read)) mask |= 1;
    close(ends[0]);
    close(ends[1]);
    return mask;
}

int fx_getaddrinfo_localhost(void)
{
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_NUMERICSERV;
    struct addrinfo *result = NULL;
    int status = getaddrinfo("localhost", "80", &hints, &result);
    if (status != 0) return -status;
    if (result == NULL) return -100;
    char host[64];
    char service[16];
    int named = getnameinfo(result->ai_addr, result->ai_addrlen, host, sizeof(host), service, sizeof(service),
                            NI_NUMERICHOST | NI_NUMERICSERV);
    int mask = 0;
    if (named == 0 && strcmp(host, "127.0.0.1") == 0) mask |= 1;
    if (named == 0 && strcmp(service, "80") == 0) mask |= 2;
    if (result->ai_family == AF_INET && result->ai_socktype == SOCK_STREAM) mask |= 4;
    if (result->ai_addr != NULL && result->ai_addrlen == sizeof(struct sockaddr_in)) mask |= 8;
    freeaddrinfo(result);
    return mask;
}

int fx_getnameinfo_numeric(void)
{
    struct sockaddr_in address;
    loopback_address(&address);
    address.sin_port = 0x5000;
    char host[64];
    char service[16];
    int status = getnameinfo((struct sockaddr *)&address, sizeof(address), host, sizeof(host), service,
                             sizeof(service), NI_NUMERICHOST | NI_NUMERICSERV);
    if (status != 0) return -status;
    return strcmp(host, "127.0.0.1") == 0 && strcmp(service, "80") == 0 ? 1 : 0;
}

int fx_timerfd_tick(void)
{
    int timer = timerfd_create(CLOCK_MONOTONIC, 0);
    if (timer < 0) return -errno;
    struct itimerspec schedule;
    memset(&schedule, 0, sizeof(schedule));
    schedule.it_value.tv_nsec = 5000000;
    schedule.it_interval.tv_nsec = 5000000;
    if (timerfd_settime(timer, 0, &schedule, NULL) != 0) return -2;
    struct pollfd request = {.fd = timer, .events = POLLIN, .revents = 0};
    if (poll(&request, 1, 2000) != 1) return -3;
    uint64_t ticks = 0;
    if (read(timer, &ticks, 8) != 8) return -4;
    close(timer);
    return ticks > 0 ? 1 : 0;
}

int fx_socketpair_shutdown(void)
{
    int pair[2];
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, pair) != 0) return -1;
    if (send(pair[0], "x", 1, 0) != 1) return -2;
    if (shutdown(pair[0], SHUT_WR) != 0) return -3;
    char byte = 0;
    int mask = 0;
    if (recv(pair[1], &byte, 1, 0) == 1) mask |= 1;
    if (recv(pair[1], &byte, 1, 0) == 0) mask |= 2;
    close(pair[0]);
    close(pair[1]);
    return mask;
}

int fx_setsockopt_roundtrip(void)
{
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) return -1;
    int value = 1;
    if (setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, &value, sizeof(value)) != 0) return -2;
    int read_back = 0;
    socklen_t length = sizeof(read_back);
    if (getsockopt(sock, SOL_SOCKET, SO_REUSEADDR, &read_back, &length) != 0) return -3;
    int type = 0;
    length = sizeof(type);
    if (getsockopt(sock, SOL_SOCKET, SO_TYPE, &type, &length) != 0) return -4;
    close(sock);
    return read_back != 0 && type == SOCK_STREAM ? 1 : 0;
}
