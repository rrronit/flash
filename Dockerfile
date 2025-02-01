# 

# Use PostgreSQL as base image
FROM postgres


# Create directories
RUN mkdir -p /sqlizer /data

# Copy the entrypoint script
COPY sql_script.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Set environment variables
ENV POSTGRES_USER=sqlizer
ENV POSTGRES_PASSWORD=sqlizer123
ENV POSTGRES_DB=sqlizer_db
ENV PGDATA=/data/pgdata

# Use custom entrypoint
ENTRYPOINT ["/entrypoint.sh"]