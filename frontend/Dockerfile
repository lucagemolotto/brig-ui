FROM nginx
COPY ./dist /usr/share/nginx/html
COPY ["nginx.conf", "/etc/nginx/conf.d/default.conf"]
ENV TARGET_HOST="192.168.2.9"
EXPOSE 9080