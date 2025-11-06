#!/bin/sh

export DATABASE_URL=sqlite://sqlite.db

cargo sqlx database setup
